use std::fs;
use std::os::unix::io::{AsRawFd, RawFd};

use crate::rom::Cartridge;
use savestate::SaveState;


pub const AS_BASE: usize = 0x100000000usize;

pub struct AddressSpace {
    pub rom_file: fs::File,
    pub extram_file: fs::File,

    pub cartridge: Cartridge,

    pub rom_bank: usize,
    pub vram_bank: usize,
    pub extram_bank: Option<usize>,
    pub extram_rw: bool,
    pub wram_bank: usize,

    pub full_vram: *mut u8,

    rom0_mapped: Option<()>,
    romn_mapped: Option<usize>,
    vram_mapped: Option<usize>,
    extram_mapped: Option<usize>,
    extram_mapped_rw: bool,
    wram0_mapped: Option<()>,
    wramn_mapped: Option<usize>,
    hram_mapped: Option<()>,

    hram_shm: Option<RawFd>,
    vram_shm: Option<RawFd>,
    wram_shm: Option<RawFd>,
}


extern "C" fn close_shm() {
    unsafe {
        libc::shm_unlink("/xgbcrew-hram\0".as_bytes().as_ptr()
                             as *const libc::c_char);

        libc::shm_unlink("/xgbcrew-vram\0".as_bytes().as_ptr()
                             as *const libc::c_char);

        libc::shm_unlink("/xgbcrew-wram\0".as_bytes().as_ptr()
                             as *const libc::c_char);
    }
}


pub trait U8Split {
    fn split_into_u8<F: FnMut(u16, u8)>(self, base_addr: u16, func: F);
    fn construct_from_u8<F: FnMut(u16) -> u8>(base_addr: u16, func: F)
        -> Self;
}

impl U8Split for u8 {
    fn split_into_u8<F: FnMut(u16, u8)>(self, base_addr: u16, mut func: F) {
        func(base_addr, self);
    }

    fn construct_from_u8<F: FnMut(u16) -> u8>(base_addr: u16, mut func: F)
        -> Self
    {
        func(base_addr)
    }
}

impl U8Split for u16 {
    fn split_into_u8<F: FnMut(u16, u8)>(self, base_addr: u16, mut func: F) {
        func(base_addr, self as u8);
        func(base_addr.wrapping_add(1u16), (self >> 8) as u8);
    }

    fn construct_from_u8<F: FnMut(u16) -> u8>(base_addr: u16, mut func: F)
        -> Self
    {
        (func(base_addr) as u16) |
            ((func(base_addr.wrapping_add(1u16)) as u16) << 8)
    }
}


impl AddressSpace {
    pub fn new(rom_path: &String, ram_path: &String) -> Self {
        Self::register_shm_unlink_handler();

        let mut addr_space = Self {
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

            full_vram: std::ptr::null_mut(),

            rom0_mapped: None,
            romn_mapped: None,
            vram_mapped: None,
            extram_mapped: None,
            extram_mapped_rw: false,
            wram0_mapped: None,
            wramn_mapped: None,
            hram_mapped: None,

            hram_shm: None,
            vram_shm: None,
            wram_shm: None,
        };

        /* I/O must be mapped for further system initalization */
        addr_space.ensure_hram_shm();
        addr_space.map_hram();

        addr_space
    }

    fn mmap(addr: usize, fd: libc::c_int, offset: usize, size: usize,
            prot: libc::c_int, flags: libc::c_int, zero: bool)
        -> *mut libc::c_void
    {
        let res = unsafe {
            libc::mmap(addr as *mut libc::c_void, size, prot, flags, fd,
                       offset as libc::off_t)
        };

        if res == libc::MAP_FAILED {
            panic!("mmap() failed");
        }

        if zero {
            unsafe {
                libc::memset(res as *mut libc::c_void, 0, size);
            }
        }

        res
    }

    fn munmap(addr: usize, size: usize) {
        let res = unsafe {
            libc::munmap(addr as *mut libc::c_void, size)
        };

        if res != 0 {
            panic!("munmap() failed");
        }
    }

    fn mprotect(addr: usize, size: usize, prot: libc::c_int) {
        let res = unsafe {
            libc::mprotect(addr as *mut libc::c_void, size, prot)
        };

        if res != 0 {
            panic!("mprotect() failed");
        }
    }

    fn map_rom0(&mut self) {
        if self.rom0_mapped.is_some() {
            return;
        }

        Self::mmap(AS_BASE, self.rom_file.as_raw_fd(), 0, 0x4000,
                   libc::PROT_READ, libc::MAP_PRIVATE | libc::MAP_FIXED,
                   false);
        self.rom0_mapped = Some(());
    }

    pub fn remap_romn(&mut self) {
        if let Some(bank) = self.romn_mapped {
            if bank == self.rom_bank {
                return;
            }
            Self::munmap(AS_BASE + 0x4000, 0x4000);
        }

        Self::mmap(AS_BASE + 0x4000, self.rom_file.as_raw_fd(),
                   self.rom_bank * 0x4000usize, 0x4000,
                   libc::PROT_READ, libc::MAP_PRIVATE | libc::MAP_FIXED, false);
        self.romn_mapped = Some(self.rom_bank);
    }

    pub fn remap_vram(&mut self) {
        if let Some(bank) = self.vram_mapped {
            if bank == self.vram_bank {
                return;
            }
            Self::munmap(AS_BASE + 0x8000, 0x2000);
        }

        Self::mmap(AS_BASE + 0x8000, self.vram_shm.unwrap(),
                   self.vram_bank * 0x2000usize, 0x2000,
                   libc::PROT_READ | libc::PROT_WRITE,
                   libc::MAP_SHARED | libc::MAP_FIXED, false);
        self.vram_mapped = Some(self.vram_bank);
    }

    pub fn remap_extram(&mut self) {
        if self.extram_mapped == self.extram_bank &&
           self.extram_mapped_rw == self.extram_rw
        {
            return;
        }

        let prot =
            if self.extram_rw {
                libc::PROT_READ | libc::PROT_WRITE
            } else {
                libc::PROT_READ
            };

        if self.extram_mapped == self.extram_bank {
            Self::mprotect(AS_BASE + 0xa000, 0x2000, prot);
            self.extram_mapped_rw = self.extram_rw;
            return;
        }

        if self.extram_mapped.is_some() {
            Self::munmap(AS_BASE + 0xa000, 0x2000);
        }
        if let Some(bank) = self.extram_bank {
            if bank == -1isize as usize {
                Self::mmap(AS_BASE + 0xa000, -1, 0, 0x2000,
                           prot,
                           libc::MAP_PRIVATE | libc::MAP_FIXED |
                           libc::MAP_ANONYMOUS,
                           false);
            } else {
                Self::mmap(AS_BASE + 0xa000, self.extram_file.as_raw_fd(),
                           bank * 0x2000usize, 0x2000,
                           prot, libc::MAP_SHARED | libc::MAP_FIXED, false);
            }
        }
        self.extram_mapped = self.extram_bank;
        self.extram_mapped_rw = self.extram_rw;
    }

    fn register_shm_unlink_handler() {
        let res = unsafe {
            libc::atexit(close_shm)
        };
        if res < 0 {
            panic!("Registering SHM region unlink handler failed");
        }
    }

    fn create_shm(name: &str, size: usize) -> RawFd {
        let shmfd = unsafe {
            libc::shm_open(name.as_bytes().as_ptr() as *const libc::c_char,
                           libc::O_RDWR | libc::O_CREAT,
                           0o755)
        };
        if shmfd < 0 {
            panic!("Creating SHM region failed");
        }

        let res = unsafe {
            libc::ftruncate(shmfd, size as libc::off_t)
        };
        if res < 0 {
            panic!("Truncating SHM region failed");
        }

        shmfd
    }

    fn ensure_hram_shm(&mut self) {
        if self.hram_shm.is_none() {
            self.hram_shm = Some(Self::create_shm("/xcgbcrew-hram\0", 0x1000));

            /* Clear HRAM */
            let hram = Self::mmap(0, self.hram_shm.unwrap(), 0, 0x1000,
                                  libc::PROT_WRITE, libc::MAP_SHARED, true)
                       as usize;
            Self::munmap(hram, 0x1000);
        }
    }

    fn ensure_vram_shm(&mut self) {
        if self.vram_shm.is_none() {
            self.vram_shm = Some(Self::create_shm("/xcgbcrew-vram\0", 0x4000));

            self.full_vram = Self::mmap(0, self.vram_shm.unwrap(), 0, 0x4000,
                                        libc::PROT_READ | libc::PROT_WRITE,
                                        libc::MAP_SHARED, true)
                             as *mut u8;
        }
    }

    fn ensure_wram_shm(&mut self) {
        if self.wram_shm.is_none() {
            self.wram_shm = Some(Self::create_shm("/xcgbcrew-wram\0", 0x8000));

            /* Clear WRAM */
            let wram = Self::mmap(0, self.wram_shm.unwrap(), 0, 0x8000,
                                  libc::PROT_WRITE, libc::MAP_SHARED, true)
                       as usize;
            Self::munmap(wram, 0x8000);
        }
    }

    fn map_wram0(&mut self) {
        if self.wram0_mapped.is_some() {
            return;
        }

        Self::mmap(AS_BASE + 0xc000, self.wram_shm.unwrap(), 0, 0x1000,
                   libc::PROT_READ | libc::PROT_WRITE,
                   libc::MAP_SHARED | libc::MAP_FIXED, false);
        Self::mmap(AS_BASE + 0xe000, self.wram_shm.unwrap(), 0, 0x1000,
                   libc::PROT_READ | libc::PROT_WRITE,
                   libc::MAP_SHARED | libc::MAP_FIXED, false);
        self.wram0_mapped = Some(());
    }

    pub fn remap_wramn(&mut self) {
        if let Some(bank) = self.wramn_mapped {
            if bank == self.wram_bank {
                return;
            }
            Self::munmap(AS_BASE + 0xd000, 0x1000);
        }
        Self::mmap(AS_BASE + 0xd000, self.wram_shm.unwrap(),
                   self.wram_bank * 0x1000usize, 0x1000,
                   libc::PROT_READ | libc::PROT_WRITE,
                   libc::MAP_SHARED | libc::MAP_FIXED, false);
        self.wramn_mapped = Some(self.wram_bank);
    }

    fn map_hram(&mut self) {
        if self.hram_mapped.is_some() {
            return;
        }

        /*
         * Theoretically, this belongs to 0xf000.  However, this
         * region is also used as a WRAM mirror (albeit accesses to
         * those portions are illegal, technically), and for I/O, the
         * former of which we should and the latter of which we must
         * catch.
         * I haven't yet got an idea how to map this to 0xf000 and at
         * least catch accesses in 0xff00..0xff7f (the I/O area).  The
         * DRs only allow protecting a region up to 32 bytes (4 * 8),
         * but we need at least 128.
         * Well, we can map everything as read-only (with an R/W
         * mirror somewhere else).  That's kind of stupid and it's
         * just plain wrong for the mirrored WRAM region, but then
         * again it's also plain wrong for games to access that
         * region.
         */
        Self::mmap(AS_BASE + 0x10000, self.hram_shm.unwrap(), 0, 0x1000,
                   libc::PROT_READ | libc::PROT_WRITE,
                   libc::MAP_SHARED | libc::MAP_FIXED, false);
        Self::mmap(AS_BASE + 0xf000, self.hram_shm.unwrap(), 0, 0x1000,
                   libc::PROT_READ,
                   libc::MAP_SHARED | libc::MAP_FIXED, false);
        self.hram_mapped = Some(());
    }

    pub fn map(&mut self) {
        self.map_rom0();
        self.remap_romn();

        self.ensure_vram_shm();
        self.remap_vram();

        self.remap_extram();

        self.ensure_wram_shm();
        self.map_wram0();
        self.remap_wramn();

        self.ensure_hram_shm();
        self.map_hram();
    }

    pub fn rom_write(&mut self, addr: u16, val: u8) {
        Cartridge::cart_write(self, addr, val);
    }

    pub fn extram_write(&mut self, addr: u16, val: u8) {
        Cartridge::cart_write(self, addr, val);
    }

    fn export_shm<T: std::io::Write>(fd: RawFd, size: usize, stream: &mut T) {
        let mapping = Self::mmap(0, fd, 0, size, libc::PROT_READ,
                                 libc::MAP_SHARED, false) as *const u8;
        let slice = unsafe {
            std::slice::from_raw_parts(mapping, size)
        };
        stream.write_all(&slice).unwrap();
        Self::munmap(mapping as usize, size);
    }

    fn import_shm<T: std::io::Read>(fd: RawFd, size: usize, stream: &mut T) {
        let mapping = Self::mmap(0, fd, 0, size, libc::PROT_WRITE,
                                 libc::MAP_SHARED, false) as *mut u8;
        let mut slice = unsafe {
            std::slice::from_raw_parts_mut(mapping, size)
        };
        stream.read_exact(&mut slice).unwrap();
        Self::munmap(mapping as usize, size);
    }
}


/* Of course, this will only cover the current area */
pub fn get_raw_read_addr(ptr: u16) -> usize {
    let mem_addr = AS_BASE + (ptr as usize);

    if ptr < 0xe000u16 {
        mem_addr
    } else if ptr < 0xfe00u16 {
        mem_addr - 0x2000
    } else if ptr < 0xfea0u16 {
        mem_addr + 0x1000
    } else if ptr < 0xff00u16 {
        mem_addr - 0x2000
    } else if ptr < 0xff80u16 {
        panic!("get_raw_read_addr() does not work for MMIO")
    } else if ptr < 0xffffu16 {
        mem_addr + 0x1000
    } else {
        panic!("get_raw_read_addr() does not work for MMIO")
    }
}


impl SaveState for AddressSpace {
    fn export<T: std::io::Write>(&self, stream: &mut T) {
        SaveState::export(&self.cartridge, stream);

        let extram_size = self.cartridge.extram_size * 0x2000;

        Self::export_shm(self.wram_shm.unwrap(), 0x8000, stream);
        Self::export_shm(self.hram_shm.unwrap(), 0x1000, stream);
        Self::export_shm(self.extram_file.as_raw_fd(), extram_size, stream);

        let vram_slice = unsafe {
            std::slice::from_raw_parts(self.full_vram, 0x4000)
        };

        stream.write_all(&vram_slice).unwrap();

        SaveState::export(self.romn_mapped.as_ref().unwrap(), stream);
        SaveState::export(self.vram_mapped.as_ref().unwrap(), stream);
        SaveState::export(&self.extram_mapped, stream);
        SaveState::export(&self.extram_mapped_rw, stream);
        SaveState::export(self.wramn_mapped.as_ref().unwrap(), stream);
    }

    fn import<T: std::io::Read>(&mut self, stream: &mut T) {
        SaveState::import(&mut self.cartridge, stream);

        let extram_size = self.cartridge.extram_size * 0x2000;

        Self::import_shm(self.wram_shm.unwrap(), 0x8000, stream);
        Self::import_shm(self.hram_shm.unwrap(), 0x1000, stream);
        Self::import_shm(self.extram_file.as_raw_fd(), extram_size, stream);

        let mut vram_slice = unsafe {
            std::slice::from_raw_parts_mut(self.full_vram, 0x4000)
        };

        stream.read_exact(&mut vram_slice).unwrap();

        SaveState::import(&mut self.rom_bank, stream);
        SaveState::import(&mut self.vram_bank, stream);
        SaveState::import(&mut self.extram_bank, stream);
        SaveState::import(&mut self.extram_rw, stream);
        SaveState::import(&mut self.wram_bank, stream);

        self.map();
    }
}
