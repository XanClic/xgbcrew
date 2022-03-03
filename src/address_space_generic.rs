#[path = "address_space/helpers.rs"]
pub mod helpers;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Seek, SeekFrom, Write};

use crate::rom::Cartridge;
use savestate::SaveState;

pub use helpers::U8Split;


pub struct AddressSpace {
    #[cfg(not(target_arch = "wasm32"))]
    pub rom_file: fs::File,
    #[cfg(not(target_arch = "wasm32"))]
    pub extram_file: fs::File,

    pub cartridge: Cartridge,

    pub rom_bank: usize,
    pub vram_bank: usize,
    pub extram_bank: Option<usize>,
    pub extram_rw: bool,
    pub wram_bank: usize,

    pub full_vram: [u8; 0x4000],

    full_hram: [u8; 0x1000],
    full_wram: [u8; 0x8000],

    #[cfg(not(target_arch = "wasm32"))]
    full_rom: Vec<u8>,
    #[cfg(target_arch = "wasm32")]
    pub full_rom: Vec<u8>,
    pub full_extram: Vec<u8>,
    virt_extram_page: [u8; 0x2000],

    pub extram_dirty: bool,
    extram_invalid: bool,
}


impl AddressSpace {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(rom_path: &String, ram_path: &String) -> Self {
        Self {
            rom_file: std::fs::OpenOptions::new()
                        .read(true)
                        .open(rom_path).unwrap(),

            extram_file: std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(ram_path).unwrap(),

            cartridge: Cartridge::new(),

            rom_bank: 1,
            vram_bank: 0,
            extram_bank: None,
            extram_rw: false,
            wram_bank: 1,

            full_vram: [0u8; 0x4000],
            full_hram: [0u8; 0x1000],
            full_wram: [0u8; 0x8000],

            full_rom: Vec::new(),
            full_extram: Vec::new(),
            virt_extram_page: [0u8; 0x2000],

            extram_dirty: false,
            extram_invalid: true,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(rom: Vec<u8>) -> Self {
        Self {
            cartridge: Cartridge::new(),

            rom_bank: 1,
            vram_bank: 0,
            extram_bank: None,
            extram_rw: false,
            wram_bank: 1,

            full_vram: [0u8; 0x4000],
            full_hram: [0u8; 0x1000],
            full_wram: [0u8; 0x8000],

            full_rom: rom,
            full_extram: Vec::new(),
            virt_extram_page: [0u8; 0x2000],

            extram_dirty: false,
            extram_invalid: true,
        }
    }

    pub fn remap_romn(&mut self) {
    }

    pub fn remap_vram(&mut self) {
    }

    pub fn remap_extram(&mut self) {
    }

    pub fn remap_wramn(&mut self) {
    }

    pub fn map(&mut self) {
        let rom_size = self.cartridge.rom_size * 0x4000;

        self.full_rom.resize(rom_size, 0);

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.rom_file.seek(SeekFrom::Start(0)).unwrap();
            self.rom_file.read_exact(self.full_rom.as_mut_slice()).unwrap();
        }

        let extram_size = self.cartridge.extram_size * 0x2000;

        self.full_extram.resize(extram_size, 0);

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.extram_file.seek(SeekFrom::Start(0)).unwrap();
            self.extram_file.read_exact(self.full_extram.as_mut_slice()).unwrap();
        }

        #[cfg(target_arch = "wasm32")]
        self.read_wasm_sav();

        self.extram_invalid = false;
    }

    #[cfg(target_arch = "wasm32")]
    fn read_wasm_sav(&mut self) -> Option<()> {
        let window = web_sys::window()?;
        let ls = window.local_storage().ok()??;

        let sav = ls.get_item(&self.cartridge.name).ok()??;
        self.full_extram = base64::decode(sav).ok()?;

        Some(())
    }

    #[cfg(target_arch = "wasm32")]
    fn write_wasm_sav(&self) -> Option<()> {
        if self.extram_invalid {
            return None;
        }

        let window = web_sys::window()?;
        let ls = window.local_storage().ok()??;

        let sav = base64::encode(&self.full_extram);
        ls.set_item(&self.cartridge.name, &sav).ok()?;

        Some(())
    }

    pub fn rom_read(&self, addr: u16) -> u8 {
        if addr < 0x4000 {
            self.full_rom[addr as usize]
        } else {
            self.full_rom[self.rom_bank * 0x4000 + (addr as usize - 0x4000)]
        }
    }

    pub fn rom_write(&mut self, addr: u16, val: u8) {
        Cartridge::cart_write(self, addr, val);
    }

    pub fn extram_read(&self, addr: u16) -> u8 {
        if let Some(bank) = self.extram_bank {
            if bank == (-1isize as usize) {
                self.virt_extram_page[addr as usize - 0xa000]
            } else {
                self.full_extram[bank * 0x2000 + (addr as usize - 0xa000)]
            }
        } else {
            0
        }
    }

    pub fn extram_write(&mut self, addr: u16, val: u8) {
        if let Some(bank) = self.extram_bank {
            if bank == (-1isize as usize) {
                Cartridge::cart_write(self, addr, val);
            } else if self.extram_rw {
                let full_ofs = bank * 0x2000 + (addr as usize - 0xa000);
                self.full_extram[full_ofs] = val;

                /* TODO: Batch writes, perhaps per frame? */
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.extram_file.seek(SeekFrom::Start(full_ofs as u64))
                                    .unwrap();
                    self.extram_file.write_all(&[val]).unwrap();
                }

                #[cfg(target_arch = "wasm32")]
                {
                    self.extram_dirty = true;
                }
            }
        }
    }

    pub fn flush_extram(&mut self) {
        #[cfg(target_arch = "wasm32")]
        self.write_wasm_sav();

        #[cfg(not(target_arch = "wasm32"))]
        unreachable!();
    }

    pub fn set_virtual_extram(&mut self, val: u8) {
        assert!(self.extram_bank == Some(-1isize as usize));

        for x in self.virt_extram_page.iter_mut() {
            *x = val;
        }
    }

    pub fn wram_read(&self, addr: u16) -> u8 {
        if addr < 0xd000 {
            self.full_wram[addr as usize - 0xc000]
        } else {
            self.full_wram[self.wram_bank * 0x1000 + (addr as usize - 0xd000)]
        }
    }

    pub fn wram_write(&mut self, addr: u16, val: u8) {
        if addr < 0xd000 {
            self.full_wram[addr as usize - 0xc000] = val;
        } else {
            self.full_wram[self.wram_bank * 0x1000 + (addr as usize - 0xd000)] =
                val;
        }
    }

    pub fn vram_read(&self, addr: u16) -> u8 {
        self.full_vram[self.vram_bank * 0x2000 + (addr as usize - 0x8000)]
    }

    pub fn vram_write(&mut self, addr: u16, val: u8) {
        self.full_vram[self.vram_bank * 0x2000 + (addr as usize - 0x8000)] =
            val;
    }

    pub fn hram_read(&self, addr: u16) -> u8 {
        self.full_hram[addr as usize - 0xf000]
    }

    pub fn hram_write(&mut self, addr: u16, val: u8) {
        self.full_hram[addr as usize - 0xf000] = val;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn raw_ptr(&self, addr: u16) -> *const u8 {
        if addr < 0x4000 {
            &self.full_rom[addr as usize] as *const u8
        } else if addr < 0x8000 {
            &self.full_rom[self.rom_bank * 0x4000 + (addr as usize - 0x4000)]
                as *const u8
        } else if addr < 0xa000 {
            &self.full_vram[self.vram_bank * 0x2000 + (addr as usize - 0x8000)]
                as *const u8
        } else if addr < 0xc000 {
            if let Some(bank) = self.extram_bank {
                if bank == (-1isize as usize) {
                    &self.virt_extram_page[addr as usize - 0xa000] as *const u8
                } else {
                    &self.full_extram[bank * 0x2000 + (addr as usize - 0xa000)]
                        as *const u8
                }
            } else {
                panic!("raw_ptr() tried to access extram, but none mapped");
            }
        } else if addr < 0xd000 {
            &self.full_wram[addr as usize - 0xc000] as *const u8
        } else if addr < 0xe000 {
            &self.full_wram[self.wram_bank * 0x1000 + (addr as usize - 0xd000)]
                as *const u8
        } else if addr < 0xfe00 {
            self.raw_ptr(addr - 0x2000)
        } else if addr < 0xfea0 {
            &self.full_hram[addr as usize - 0xf000] as *const u8
        } else if addr < 0xff00 {
            self.raw_ptr(addr - 0x2000)
        } else {
            &self.full_hram[addr as usize - 0xf000] as *const u8
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn read_u8(&self, addr: u16) -> u8 {
        if addr < 0x4000 {
            self.full_rom[addr as usize]
        } else if addr < 0x8000 {
            self.full_rom[self.rom_bank * 0x4000 + (addr as usize - 0x4000)]
        } else if addr < 0xa000 {
            self.full_vram[self.vram_bank * 0x2000 + (addr as usize - 0x8000)]
        } else if addr < 0xc000 {
            if let Some(bank) = self.extram_bank {
                if bank == (-1isize as usize) {
                    self.virt_extram_page[addr as usize - 0xa000]
                } else {
                    self.full_extram[bank * 0x2000 + (addr as usize - 0xa000)]
                }
            } else {
                panic!("raw_ptr() tried to access extram, but none mapped");
            }
        } else if addr < 0xd000 {
            self.full_wram[addr as usize - 0xc000]
        } else if addr < 0xe000 {
            self.full_wram[self.wram_bank * 0x1000 + (addr as usize - 0xd000)]
        } else if addr < 0xfe00 {
            self.read_u8(addr - 0x2000)
        } else if addr < 0xfea0 {
            self.full_hram[addr as usize - 0xf000]
        } else if addr < 0xff00 {
            self.read_u8(addr - 0x2000)
        } else {
            self.full_hram[addr as usize - 0xf000]
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn raw_mut_ptr(&mut self, addr: u16) -> *mut u8 {
        if addr < 0x4000 {
            &mut self.full_rom[addr as usize] as *mut u8
        } else if addr < 0x8000 {
            &mut self.full_rom[self.rom_bank * 0x4000 +
                               (addr as usize - 0x4000)]
                as *mut u8
        } else if addr < 0xa000 {
            &mut self.full_vram[self.vram_bank * 0x2000 +
                                (addr as usize - 0x8000)]
                as *mut u8
        } else if addr < 0xc000 {
            if let Some(bank) = self.extram_bank {
                if bank == (-1isize as usize) {
                    &mut self.virt_extram_page[addr as usize - 0xa000]
                        as *mut u8
                } else {
                    &mut self.full_extram[bank * 0x2000 +
                                          (addr as usize - 0xa000)]
                        as *mut u8
                }
            } else {
                panic!("raw_ptr() tried to access extram, but none mapped");
            }
        } else if addr < 0xd000 {
            &mut self.full_wram[addr as usize - 0xc000] as *mut u8
        } else if addr < 0xe000 {
            &mut self.full_wram[self.wram_bank * 0x1000 +
                                (addr as usize - 0xd000)]
                as *mut u8
        } else if addr < 0xfe00 {
            self.raw_mut_ptr(addr - 0x2000)
        } else if addr < 0xfea0 {
            &mut self.full_hram[addr as usize - 0xf000] as *mut u8
        } else if addr < 0xff00 {
            self.raw_mut_ptr(addr - 0x2000)
        } else {
            &mut self.full_hram[addr as usize - 0xf000] as *mut u8
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn write_u8(&mut self, addr: u16, val: u8) {
        if addr < 0x4000 {
            self.full_rom[addr as usize] = val;
        } else if addr < 0x8000 {
            self.full_rom[self.rom_bank * 0x4000 +
                          (addr as usize - 0x4000)]
                = val;
        } else if addr < 0xa000 {
            self.full_vram[self.vram_bank * 0x2000 +
                           (addr as usize - 0x8000)]
                = val;
        } else if addr < 0xc000 {
            if let Some(bank) = self.extram_bank {
                if bank == (-1isize as usize) {
                    self.virt_extram_page[addr as usize - 0xa000]
                        = val;
                } else {
                    self.full_extram[bank * 0x2000 +
                                     (addr as usize - 0xa000)]
                        = val;
                }
            } else {
                panic!("raw_ptr() tried to access extram, but none mapped");
            }
        } else if addr < 0xd000 {
            self.full_wram[addr as usize - 0xc000] = val;
        } else if addr < 0xe000 {
            self.full_wram[self.wram_bank * 0x1000 +
                           (addr as usize - 0xd000)]
                = val;
        } else if addr < 0xfe00 {
            self.write_u8(addr - 0x2000, val);
        } else if addr < 0xfea0 {
            self.full_hram[addr as usize - 0xf000] = val;
        } else if addr < 0xff00 {
            self.write_u8(addr - 0x2000, val);
        } else {
            self.full_hram[addr as usize - 0xf000] = val;
        }
    }
}


impl SaveState for AddressSpace {
    fn export<T: std::io::Write>(&self, stream: &mut T, version: u64) {
        SaveState::export(&self.cartridge, stream, version);

        stream.write_all(&self.full_wram).unwrap();
        stream.write_all(&self.full_hram).unwrap();

        let extram_size = self.cartridge.extram_size * 0x2000;
        if extram_size != 0 {
            stream.write_all(self.full_extram.as_slice()).unwrap();
        }

        stream.write_all(&self.full_vram).unwrap();

        SaveState::export(&self.rom_bank, stream, version);
        SaveState::export(&self.vram_bank, stream, version);
        SaveState::export(&self.extram_bank, stream, version);
        SaveState::export(&self.extram_rw, stream, version);
        SaveState::export(&self.wram_bank, stream, version);
    }

    fn import<T: std::io::Read>(&mut self, stream: &mut T, version: u64) {
        SaveState::import(&mut self.cartridge, stream, version);

        stream.read_exact(&mut self.full_wram).unwrap();
        stream.read_exact(&mut self.full_hram).unwrap();

        let extram_size = self.cartridge.extram_size * 0x2000;
        if extram_size != 0 {
            stream.read_exact(self.full_extram.as_mut_slice()).unwrap();
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.extram_file.seek(SeekFrom::Start(0)).unwrap();
                self.extram_file.write_all(self.full_extram.as_slice()).unwrap();
            }
            #[cfg(target_arch = "wasm32")]
            self.write_wasm_sav();
        }

        stream.read_exact(&mut self.full_vram).unwrap();

        SaveState::import(&mut self.rom_bank, stream, version);
        SaveState::import(&mut self.vram_bank, stream, version);
        SaveState::import(&mut self.extram_bank, stream, version);
        SaveState::import(&mut self.extram_rw, stream, version);
        SaveState::import(&mut self.wram_bank, stream, version);

        self.map();
    }
}
