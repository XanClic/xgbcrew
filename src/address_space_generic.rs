#[path = "address_space/helpers.rs"]
pub mod helpers;

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::rom::Cartridge;
use savestate::SaveState;

pub use helpers::U8Split;


pub struct AddressSpace {
    pub rom_file: fs::File,
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

    full_rom: Vec<u8>,
    full_extram: Vec<u8>,
    virt_extram_page: [u8; 0x2000],
}


impl AddressSpace {
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

        self.rom_file.seek(SeekFrom::Start(0)).unwrap();
        self.rom_file.read_exact(self.full_rom.as_mut_slice()).unwrap();

        let extram_size = self.cartridge.extram_size * 0x2000;

        self.full_extram.resize(extram_size, 0);

        self.extram_file.seek(SeekFrom::Start(0)).unwrap();
        self.extram_file.read_exact(self.full_extram.as_mut_slice()).unwrap();
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
                self.full_extram[bank * 0x2000 + (addr as usize - 0xa000)] =
                    val;
            }
        }
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
            self.extram_file.seek(SeekFrom::Start(0)).unwrap();
            self.extram_file.write_all(self.full_extram.as_slice()).unwrap();
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
