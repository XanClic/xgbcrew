use std::io::{Read, Seek, SeekFrom, Write};
use std::time::SystemTime;

use crate::address_space::AddressSpace;
use crate::io::serial::SerialConnParam;
use crate::system_state::SystemParams;


#[derive(Serialize, Deserialize, Debug)]
struct RomDataArea {
    entry: [u8; 4],
    opening_graphic_0: [u8; 0x10],
    opening_graphic_1: [u8; 0x10],
    opening_graphic_2: [u8; 0x10],
    title: [u8; 15],
    cgb_mode: u8,
    licensee_new: [u8; 2],
    sgb_mode: u8,
    cartridge: u8,
    rom_size: u8,
    extram_size: u8,
    area_code: u8,
    licensee_old: u8,
    rom_version: u8,
    ecc: u8,
    checksum: u16,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
struct RamRTCData {
    set_at: SystemTime,

    secs: u8,
    mins: u8,
    hours: u8,
    days: u16,
    halted: bool,
}

enum MbcType {
    NoMBC,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
    MMM01,
}

#[derive(SaveState)]
pub struct Cartridge {
    #[savestate(skip)]
    mbc: MbcType,
    #[savestate(skip)]
    extram: bool,
    #[savestate(skip)]
    rumble: bool,

    #[savestate(skip)]
    pub rom_size: usize,
    #[savestate(skip)]
    pub extram_size: usize,

    mbc1_ram_banking: bool,
    mbc3_hidden_ram_rw: bool,
    mbc3_clock_sel: u8,
    rtc: Option<RamRTCData>,
    rtc_latched: Option<SystemTime>,

    #[savestate(skip_if("version < 4"))]
    pub rumble_state: bool,
}

impl Cartridge {
    pub fn new() -> Self {
        Self {
            mbc: MbcType::NoMBC,
            extram: false,
            rumble: false,

            rom_size: 2,
            extram_size: 0,

            mbc1_ram_banking: false,
            mbc3_hidden_ram_rw: false,
            mbc3_clock_sel: 0,
            rtc: None,
            rtc_latched: None,

            rumble_state: false,
        }
    }

    pub fn init_map(addr_space: &mut AddressSpace) {
        let c = &mut addr_space.cartridge;

        match c.mbc {
            MbcType::NoMBC => {
                addr_space.rom_bank = 1;
                addr_space.extram_bank = None;
            },

            MbcType::MBC1 => {
                addr_space.rom_bank = 1;
                if c.extram {
                    addr_space.extram_bank = Some(0);
                    addr_space.extram_rw = false;
                } else {
                    addr_space.extram_bank = None;
                }
            },

            MbcType::MBC2 => {
                addr_space.rom_bank = 1;
                if c.extram {
                    addr_space.extram_bank = Some(0);
                    addr_space.extram_rw = false;
                } else {
                    addr_space.extram_bank = None;
                }
            },

            MbcType::MBC3 => {
                addr_space.rom_bank = 1;
                if c.extram {
                    addr_space.extram_bank = Some(0);
                    addr_space.extram_rw = false;
                } else {
                    addr_space.extram_bank = None;
                }
            },

            MbcType::MBC5 => {
                addr_space.rom_bank = 1;
                if c.extram {
                    addr_space.extram_bank = Some(0);
                    addr_space.extram_rw = false;
                } else {
                    addr_space.extram_bank = None;
                }
            },

            _ => panic!("MBC type not supported"),
        }

        addr_space.map();
    }

    fn mbc1_write(addr_space: &mut AddressSpace, addr: u16, val: u8) {
        let c = &mut addr_space.cartridge;

        match addr & 0xe000 {
            0x0000 => {
                addr_space.extram_rw = val == 0x0a;
                addr_space.remap_extram();
            },

            0x2000 => {
                let mut minor_bank = val as usize & 0x1f;
                if minor_bank == 0 {
                    minor_bank = 1;
                }

                let bank = (addr_space.rom_bank & !0x1f) | minor_bank;
                addr_space.rom_bank = bank % c.rom_size;
                addr_space.remap_romn();
            },

            0x4000 => {
                if c.mbc1_ram_banking {
                    if c.extram {
                        let bank = val as usize & 0x03;
                        addr_space.extram_bank = Some(bank % c.extram_size);
                        addr_space.remap_extram();
                    }
                } else {
                    let bank = (addr_space.rom_bank & 0x1f) |
                               ((val as usize & 0x03) << 5);
                    addr_space.rom_bank = bank % c.rom_size;
                    addr_space.remap_romn();
                }
            },

            0x6000 => {
                if val & 0x01 != 0 {
                    c.mbc1_ram_banking = true;
                } else {
                    c.mbc1_ram_banking = false;
                }
            },

            0xa000 => (),

            _ => unreachable!(),
        }
    }

    fn mbc2_write(addr_space: &mut AddressSpace, addr: u16, val: u8) {
        let c = &mut addr_space.cartridge;

        if addr & 0xf000 == 0x0000 {
            addr_space.extram_rw = val == 0x0a;
            addr_space.remap_extram();
        } else if addr & 0xff00 == 0x2100 {
            let mut bank = val as usize & 0x0f;
            if bank == 0 {
                bank = 1;
            }
            addr_space.rom_bank = bank % c.rom_size;
            addr_space.remap_romn();
        }
    }

    fn mbc3_time(&self) -> (u64, bool) {
        let rtc =
            match self.rtc.as_ref() {
                Some(x) => x,
                None => return (0, false),
            };

        let lt = match self.rtc_latched {
            Some(x) => x,
            None => SystemTime::now(),
        };

        let base = rtc.secs as u64 +
                   60 * (rtc.mins as u64 +
                         60 * (rtc.hours as u64 +
                               24 * ((rtc.days & 0x1ff) as u64)));

        let mut dc = rtc.days & (1 << 15) != 0;

        let secs = if rtc.halted {
                base
            } else {
                let s =
                    match lt.duration_since(rtc.set_at) {
                        Ok(x) => x.as_secs(),
                        Err(_) => 0,
                    };

                base + s
            };

        if secs >= 86400 * 512 {
            dc = true;
        }

        (secs % (86400 * 512), dc)
    }

    fn mbc3_write(addr_space: &mut AddressSpace, addr: u16, mut val: u8) {
        let c = &mut addr_space.cartridge;

        match addr & 0xe000 {
            0x0000 => {
                c.mbc3_hidden_ram_rw = val == 0x0a;
                if let Some(bank) = addr_space.extram_bank {
                    if bank < 4 {
                        addr_space.extram_rw = c.mbc3_hidden_ram_rw;
                    }
                }
                addr_space.remap_extram();
            },

            0x2000 => {
                let mut bank = val as usize & 0x7f;
                if bank == 0 {
                    bank = 1;
                }
                addr_space.rom_bank = bank % c.rom_size;
                addr_space.remap_romn();
            },

            0x4000 => {
                if val >= 0x08 && val <= 0x0c && c.rtc.is_none() {
                    val = val & 0x03;
                }

                if val >= 0x08 && val <= 0x0c {
                    let (secs, dc) = c.mbc3_time();
                    let halted = c.rtc.as_ref().unwrap().halted;

                    c.mbc3_clock_sel = val;

                    addr_space.extram_bank = Some(-1isize as usize);
                    /* So we can do a memset */
                    addr_space.extram_rw = true;
                    addr_space.remap_extram();

                    let x = match val {
                        0x08 => secs % 60,
                        0x09 => (secs / 60) % 60,
                        0x0a => (secs / 3600) % 24,
                        0x0b => (secs / 86400) & 0xff,
                        0x0c => ((secs / 86400) >> 8) |
                                if halted { 1 << 6 } else { 0 } |
                                if dc { 1 << 7 } else { 0 },

                        _ => unreachable!(),
                    };

                    addr_space.set_virtual_extram(x as u8);

                    /* memset is done */
                    addr_space.extram_rw = false;
                    addr_space.remap_extram();
                } else if c.extram {
                    let bank = val as usize & 0x03;
                    addr_space.extram_bank = Some(bank % c.extram_size);
                    addr_space.extram_rw = c.mbc3_hidden_ram_rw;
                    addr_space.remap_extram();
                }
            },

            0x6000 => {
                /* TODO: Stricter latching handling */
                if val != 0 {
                    c.rtc_latched = Some(SystemTime::now());
                }
            },

            0xa000 => {
                if c.rtc.is_none() {
                    return;
                }

                let (mut tsecs, mut dc) = c.mbc3_time();
                let rtc = c.rtc.as_mut().unwrap();
                let mut halted = rtc.halted;

                let secs = tsecs % 60;
                let mins = (tsecs / 60) % 60;
                let hours = (tsecs / 3600) % 24;
                let days = tsecs / 86400;

                match c.mbc3_clock_sel {
                    0x08 => {
                        tsecs = val as u64 +
                                60 * (mins +
                                      60 * (hours +
                                            24 * days));
                    },

                    0x09 => {
                        tsecs = secs +
                                60 * (val as u64 +
                                      60 * (hours +
                                            24 * days));
                    },

                    0x0a => {
                        tsecs = secs +
                                60 * (mins +
                                      60 * (val as u64 +
                                            24 * days));
                    },

                    0x0b => {
                        tsecs = secs +
                                60 * (mins +
                                      60 * (hours +
                                            24 * (val as u64 |
                                                  (days & 0x100))));
                    },

                    0x0c => {
                        tsecs = secs +
                                60 * (mins +
                                      60 * (hours +
                                            24 * ((days & 0xff) |
                                                  ((val as u64 & 0x01)
                                                   << 8))));

                        halted = val & (1 << 6) != 0;
                        dc = val & (1 << 7) != 0;
                    },

                    _ => unreachable!(),
                };

                tsecs %= 86400 * 512;
                if dc {
                    tsecs += 86400 * 512;
                }

                rtc.set_at = SystemTime::now();

                rtc.secs = (tsecs % 60) as u8;
                rtc.mins = ((tsecs / 60) % 60) as u8;
                rtc.hours = ((tsecs / 3600) % 24) as u8;
                rtc.days = ((tsecs / 86400) & 0x3ff) as u16;
                rtc.halted = halted;

                let pos = c.extram_size * 8192;
                addr_space.extram_file.seek(SeekFrom::Start(pos as u64))
                                      .unwrap();

                let raw_rtc_data = bincode::serialize(&rtc).unwrap();
                addr_space.extram_file.write_all(&raw_rtc_data).unwrap();

                /* So we can do a memset */
                addr_space.extram_rw = true;
                addr_space.remap_extram();

                addr_space.set_virtual_extram(val);

                /* memset is done */
                addr_space.extram_rw = false;
                addr_space.remap_extram();
            },

            _ => unreachable!(),
        }
    }

    fn mbc5_write(addr_space: &mut AddressSpace, addr: u16, val: u8) {
        let c = &mut addr_space.cartridge;

        match addr & 0xf000 {
            0x0000 | 0x1000 => {
                addr_space.extram_rw = val == 0x0a;
                addr_space.remap_extram();
            },

            0x2000 => {
                let mut bank = (val as usize) |
                               (addr_space.rom_bank & 0xff00);
                if bank == 0 {
                    bank = 1;
                }
                addr_space.rom_bank = bank % c.rom_size;
                addr_space.remap_romn();
            },

            0x3000 => {
                let mut bank = (addr_space.rom_bank & 0x00ff) |
                               ((val as usize & 0x01) << 8);
                if bank == 0 {
                    bank = 1;
                }
                addr_space.rom_bank = bank % c.rom_size;
                addr_space.remap_romn();
            },

            0x4000 | 0x5000 => {
                if c.rumble {
                    c.rumble_state = val & (1 << 3) != 0;
                }

                if c.extram {
                    let mask = if c.rumble { 0x07 } else { 0x0f };
                    let bank = val as usize & mask;
                    addr_space.extram_bank = Some(bank % c.extram_size);
                    addr_space.remap_extram();
                }
            },

            0x6000 | 0x7000 | 0xa000 | 0xb000 => (),

            _ => panic!("{:04x}", addr),
        }
    }

    pub fn cart_write(addr_space: &mut AddressSpace, addr: u16, val: u8) {
        match addr_space.cartridge.mbc {
            MbcType::MBC1 => Cartridge::mbc1_write(addr_space, addr, val),
            MbcType::MBC2 => Cartridge::mbc2_write(addr_space, addr, val),
            MbcType::MBC3 => Cartridge::mbc3_write(addr_space, addr, val),
            MbcType::MBC5 => Cartridge::mbc5_write(addr_space, addr, val),

            _ => println!("ROM write {:02x} => {:04x} not handled", val, addr),
        }
    }
}


pub fn load_rom(addr_space: &mut AddressSpace) -> SystemParams {
    addr_space.rom_file.seek(SeekFrom::Start(0x100)).unwrap();

    let mut raw_rda: [u8; 0x50] = [0u8; 0x50];
    addr_space.rom_file.read_exact(&mut raw_rda).unwrap();

    let rom_data_area: RomDataArea =
        bincode::deserialize(&raw_rda).unwrap();

    let (mbc, extram, batt, rtc, rumble) = match rom_data_area.cartridge {
        0x00 => (MbcType::NoMBC, false, false, false, false),
        0x01 => (MbcType::MBC1,  false, false, false, false),
        0x02 => (MbcType::MBC1,   true, false, false, false),
        0x03 => (MbcType::MBC1,   true,  true, false, false),

        0x05 => (MbcType::MBC2,  false, false, false, false),
        0x06 => (MbcType::MBC2,  false,  true, false, false),

        0x08 => (MbcType::NoMBC,  true, false, false, false),
        0x09 => (MbcType::NoMBC,  true,  true, false, false),

        0x0b => (MbcType::MMM01, false, false, false, false),
        0x0c => (MbcType::MMM01,  true, false, false, false),
        0x0d => (MbcType::MMM01,  true,  true, false, false),

        0x0f => (MbcType::MBC3,  false,  true,  true, false),
        0x10 => (MbcType::MBC3,   true,  true,  true, false),
        0x11 => (MbcType::MBC3,  false, false, false, false),
        0x12 => (MbcType::MBC3,   true, false, false, false),
        0x13 => (MbcType::MBC3,   true,  true, false, false),

        0x19 => (MbcType::MBC5,  false, false, false, false),
        0x1a => (MbcType::MBC5,   true, false, false, false),
        0x1b => (MbcType::MBC5,   true,  true, false, false),
        0x1c => (MbcType::MBC5,  false, false, false,  true),
        0x1d => (MbcType::MBC5,  false,  true, false,  true),
        0x1e => (MbcType::MBC5,  false,  true,  true,  true),

        _ => panic!("Unknown cartridge type {:#x}", rom_data_area.cartridge),
    };

    let rom_size = match rom_data_area.rom_size {
        0..=9 => 2usize << rom_data_area.rom_size,
        0x52  => 72usize,
        0x53  => 80usize,
        0x54  => 96usize,

        _ => panic!("Invalid ROM size"),
    };

    let extram_size = match rom_data_area.extram_size {
        0 => 0usize,
        1 | 2 => 1usize,
        3 => 4usize,
        4 => 16usize,

        _ => panic!("Invalid external RAM size"),
    };

    let gbc_mode = rom_data_area.cgb_mode & 0x80 != 0;
    let sgb_mode = rom_data_area.sgb_mode == 0x03;
    let cart_name = String::from_utf8_lossy(&rom_data_area.title).to_string();

    print!("{}, ", cart_name.replace("\0", "."));
    if gbc_mode && sgb_mode {
        print!("GBC+SGB");
    } else if gbc_mode {
        print!("GBC");
    } else if sgb_mode {
        print!("SGB");
    } else {
        print!("GB");
    }

    println!(", {} kB ROM, {} kB external RAM",
             rom_size * 16, extram_size * 8);

    println!("Cartridge type: ROM{}{}{}{}{}",
             match mbc {
                 MbcType::NoMBC => "",
                 MbcType::MBC1  => "+MBC1",
                 MbcType::MBC2  => "+MBC2",
                 MbcType::MBC3  => "+MBC3",
                 MbcType::MBC5  => "+MBC5",
                 MbcType::MMM01 => "+MMM01",
             },
             if extram { "+EXTRAM" } else { "" },
             if batt { "+BATTERY" } else { "" },
             if rtc { "+RTC" } else { "" },
             if rumble { "+RUMBLE" } else { "" });


    /* FIXME: Can you get this statically? */
    let rtc_data_length =
        bincode::serialize(&RamRTCData {
            set_at: SystemTime::UNIX_EPOCH,

            secs: 0,
            mins: 0,
            hours: 0,
            days: 0,
            halted: true,
        }).unwrap().len();

    let mut extram_len = extram_size * 8192;
    if rtc {
        extram_len += rtc_data_length;
    }

    if cfg!(target_os = "linux") {
        addr_space.extram_file.set_len(extram_len as u64).unwrap();
    } else {
        let cur_len = addr_space.extram_file.seek(SeekFrom::End(0)).unwrap();

        if cur_len < extram_len as u64 {
            let mut empty = Vec::<u8>::new();
            empty.resize(extram_len - cur_len as usize, 0u8);
            addr_space.extram_file.write_all(&empty).unwrap();
        }
    }

    let rtc_data = if rtc {
            let pos = extram_size * 8192;
            addr_space.extram_file.seek(SeekFrom::Start(pos as u64)).unwrap();

            let mut raw_rtc_data = Vec::<u8>::new();
            raw_rtc_data.resize(rtc_data_length, 0u8);
            addr_space.extram_file.read_exact(&mut raw_rtc_data).unwrap();

            Some(bincode::deserialize::<RamRTCData>(&raw_rtc_data).unwrap())
        } else {
            None
        };

    addr_space.cartridge = Cartridge {
        mbc: mbc,
        extram: extram || batt,
        rumble: rumble,

        rom_size: rom_size,
        extram_size: extram_size,

        mbc1_ram_banking: false,
        mbc3_hidden_ram_rw: false,
        mbc3_clock_sel: 0,
        rtc: rtc_data,
        rtc_latched: None,

        rumble_state: false,
    };

    Cartridge::init_map(addr_space);

    SystemParams {
        cgb: gbc_mode,
        sgb: sgb_mode,
        cartridge_name: cart_name,
        serial_conn_param: SerialConnParam::default(),
    }
}
