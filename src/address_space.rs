use std::fs;
use std::os::unix::io::{AsRawFd, RawFd};


const AS_BASE: usize = 0x100000000usize;

pub struct AddressSpace {
    pub rom_file: fs::File,
    pub extram_file: fs::File,

    pub rom_bank: usize,
    pub vram_bank: usize,
    pub extram_bank: Option<usize>,
    pub wram_bank: usize,

    rom0_mapped: Option<()>,
    romn_mapped: Option<usize>,
    vram_mapped: Option<usize>,
    extram_mapped: Option<usize>,
    wram0_mapped: Option<()>,
    wramn_mapped: Option<usize>,
    hram_mapped: Option<()>,

    hram_shm: Option<RawFd>,
    vram_shm: Option<RawFd>,
    wram_shm: Option<RawFd>,
}


extern "C" fn close_shm() {
    unsafe {
        libc::shm_unlink("/xgbcrew-wram\0".as_bytes().as_ptr()
                             as *const libc::c_char);
    }
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

            rom_bank: 1,
            vram_bank: 0,
            extram_bank: None,
            wram_bank: 1,

            rom0_mapped: None,
            romn_mapped: None,
            vram_mapped: None,
            extram_mapped: None,
            wram0_mapped: None,
            wramn_mapped: None,
            hram_mapped: None,

            hram_shm: None,
            vram_shm: None,
            wram_shm: None,
        }
    }

    fn mmap(&mut self, addr: usize, fd: libc::c_int, offset: usize, size: usize,
            prot: libc::c_int, flags: libc::c_int)
    {
        let res = unsafe {
            libc::mmap(addr as *mut libc::c_void, size, prot, flags, fd,
                       offset as libc::off_t)
        };

        if res == libc::MAP_FAILED {
            panic!("mmap() failed");
        }
    }

    fn munmap(&mut self, addr: usize, size: usize) {
        let res = unsafe {
            libc::munmap(addr as *mut libc::c_void, size)
        };

        if res != 0 {
            panic!("munmap() failed");
        }
    }

    fn map_rom0(&mut self) {
        if self.rom0_mapped.is_some() {
            return;
        }

        self.mmap(AS_BASE, self.rom_file.as_raw_fd(), 0, 0x4000,
                  libc::PROT_READ, libc::MAP_PRIVATE | libc::MAP_FIXED);
        self.rom0_mapped = Some(());
    }

    fn remap_romn(&mut self) {
        if let Some(bank) = self.romn_mapped {
            if bank == self.rom_bank {
                return;
            }
            self.munmap(AS_BASE + 0x4000, 0x4000);
        }

        self.mmap(AS_BASE + 0x4000, self.rom_file.as_raw_fd(),
                  self.rom_bank * 0x4000usize, 0x4000,
                  libc::PROT_READ, libc::MAP_PRIVATE | libc::MAP_FIXED);
        self.romn_mapped = Some(self.rom_bank);
    }

    fn map_vram(&mut self) {
        if let Some(bank) = self.vram_mapped {
            if bank == self.vram_bank {
                return;
            }
            self.munmap(AS_BASE + 0x8000, 0x2000);
        }

        self.mmap(AS_BASE + 0x8000, self.vram_shm.unwrap(),
                  self.vram_bank * 0x2000usize, 0x2000,
                  libc::PROT_READ | libc::PROT_WRITE,
                  libc::MAP_PRIVATE | libc::MAP_FIXED |
                  libc::MAP_ANONYMOUS);
        self.vram_mapped = Some(self.vram_bank);
    }

    fn remap_extram(&mut self) {
        if self.extram_mapped == self.extram_bank {
            return;
        }
        if self.extram_mapped.is_some() {
            self.munmap(AS_BASE + 0xa000, 0x2000);
        }
        if let Some(bank) = self.extram_bank {
            self.mmap(AS_BASE + 0xa000, self.extram_file.as_raw_fd(),
                      bank * 0x2000usize, 0x2000,
                      libc::PROT_READ | libc::PROT_WRITE,
                      libc::MAP_SHARED | libc::MAP_FIXED);
        }
        self.extram_mapped = self.extram_bank;
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
        }
    }

    fn ensure_vram_shm(&mut self) {
        if self.vram_shm.is_none() {
            self.vram_shm = Some(Self::create_shm("/xcgbcrew-vram\0", 0x4000));
        }
    }

    fn ensure_wram_shm(&mut self) {
        if self.wram_shm.is_none() {
            self.wram_shm = Some(Self::create_shm("/xcgbcrew-wram\0", 0x8000));
        }
    }

    fn map_wram0(&mut self) {
        if self.wram0_mapped.is_some() {
            return;
        }

        self.mmap(AS_BASE + 0xc000, self.wram_shm.unwrap(), 0, 0x1000,
                  libc::PROT_READ | libc::PROT_WRITE,
                  libc::MAP_SHARED | libc::MAP_FIXED);
        self.mmap(AS_BASE + 0xe000, self.wram_shm.unwrap(), 0, 0x1000,
                  libc::PROT_READ | libc::PROT_WRITE,
                  libc::MAP_SHARED | libc::MAP_FIXED);
        self.wram0_mapped = Some(());
    }

    fn remap_wramn(&mut self) {
        if let Some(bank) = self.wramn_mapped {
            if bank == self.wram_bank {
                return;
            }
            self.munmap(AS_BASE + 0xd000, 0x1000);
        }
        self.mmap(AS_BASE + 0xd000, self.wram_shm.unwrap(),
                  self.wram_bank * 0x1000usize, 0x1000,
                  libc::PROT_READ | libc::PROT_WRITE,
                  libc::MAP_SHARED | libc::MAP_FIXED);
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
        self.mmap(AS_BASE + 0x10000, self.hram_shm.unwrap(), 0, 0x1000,
                  libc::PROT_READ | libc::PROT_WRITE,
                  libc::MAP_SHARED | libc::MAP_FIXED);
        self.mmap(AS_BASE + 0xf000, self.hram_shm.unwrap(), 0, 0x1000,
                  libc::PROT_READ,
                  libc::MAP_SHARED | libc::MAP_FIXED);
        self.hram_mapped = Some(());
    }

    pub fn map(&mut self) {
        Self::register_shm_unlink_handler();

        self.map_rom0();
        self.remap_romn();

        self.ensure_vram_shm();
        self.map_vram();

        self.remap_extram();

        self.ensure_wram_shm();
        self.map_wram0();
        self.remap_wramn();

        self.ensure_hram_shm();
        self.map_hram();
    }
}
