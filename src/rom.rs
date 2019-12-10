use std::io::{Read, Seek, SeekFrom};
use crate::address_space::AddressSpace;
use crate::system_state::SystemState;


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

enum MbcType {
    NoMBC,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
    MMM01,
}


pub fn load_rom(addr_space: &mut AddressSpace, state: &mut SystemState) {
    addr_space.rom_file.seek(SeekFrom::Start(0x100)).unwrap();

    let mut raw_rda: [u8; 0x50] = [0u8; 0x50];
    if addr_space.rom_file.read(&mut raw_rda).unwrap() < 0x50 {
        panic!("Short read");
    }

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
        0..=6 => 2 << rom_data_area.rom_size,
        0x52  => 72,
        0x53  => 80,
        0x54  => 96,

        _ => panic!("Invalid ROM size"),
    };

    let extram_size = match rom_data_area.extram_size {
        0 => 0,
        1 | 2 => 1,
        3 => 4,
        4 => 16,

        _ => panic!("Invalid external RAM size"),
    };

    let gbc_mode = rom_data_area.cgb_mode & 0x80 != 0;
    let sgb_mode = rom_data_area.sgb_mode == 0x03;

    print!("{}, ",
           String::from_utf8_lossy(&rom_data_area.title).replace("\0",
                                                                 "."));
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


    addr_space.extram_file.set_len(extram_size * 8192).unwrap();

    state.cgb = gbc_mode;
}
