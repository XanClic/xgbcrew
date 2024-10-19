#[macro_export]
#[cfg(target_os = "linux")]
macro_rules! mem {
    [$ss:expr; read $a:expr] => (
        mem!($ss; $a)
    );

    [$ss:expr; write $v:expr => $a:expr] => (
        mem!($ss; $v => $a)
    );

    ($ss:expr; $v:expr => $a:expr) => {
        {
            let system_state: &mut $crate::system_state::SystemState = $ss;
            let gb_addr: u16 = $a;
            let value: u8 = $v;

            unsafe {
                let mem_addr = AS_BASE + (gb_addr as usize);

                if gb_addr < 0x8000 {
                    /* ROM */
                    system_state.addr_space.rom_write(gb_addr, value);
                } else if gb_addr < 0xe000 {
                    if !(0xa000..0xc000).contains(&gb_addr) {
                        /* Video/Working RAM */
                        *(mem_addr as *mut u8) = value;
                    } else {
                        /* External RAM */
                        if system_state.addr_space.extram_rw {
                            *(mem_addr as *mut u8) = value;
                        } else {
                            system_state.addr_space.extram_write(gb_addr, value);
                        }
                    }
                } else if (0xff80..0xffff).contains(&gb_addr) {
                    /* High WRAM and stack */
                    *((mem_addr + 0x1000) as *mut u8) = value;
                } else if gb_addr >= 0xfea0 {
                    if gb_addr >= 0xff00 {
                        /* I/O */
                        io_write(system_state, gb_addr - 0xff00, value);
                    } else {
                        /* Illegal to access, mirror WRAM */
                        *((mem_addr - 0x2000) as *mut u8) = value;
                    }
                } else if gb_addr >= 0xfe00 {
                    /* OAM */
                    *((mem_addr + 0x1000) as *mut u8) = value;
                } else {
                    /* Illegal to access, mirror WRAM */
                    *((mem_addr - 0x2000) as *mut u8) = value;
                }
            }
        }
    };

    [$ss:expr; $v:expr => $a:expr] => (
        mem!($ss; $v => $a)
    );

    ($ss:expr; $a:expr) => {
        {
            let system_state: &mut $crate::system_state::SystemState = $ss;
            let gb_addr: u16 = $a;

            unsafe {
                let mem_addr = AS_BASE + (gb_addr as usize);

                if gb_addr < 0xe000 {
                    /* Normal AS */
                    *(mem_addr as *const u8)
                } else if (0xff80..0xffff).contains(&gb_addr) {
                    /* High WRAM and stack */
                    *((mem_addr + 0x1000) as *const u8)
                } else if gb_addr >= 0xfea0 {
                    if gb_addr >= 0xff00 {
                        /* I/O */
                        io_read(system_state, gb_addr - 0xff00)
                    } else {
                        /* Illegal to access, mirror WRAM */
                        *((mem_addr - 0x2000) as *const u8)
                    }
                } else if gb_addr >= 0xfe00 {
                    /* OAM */
                    *((mem_addr + 0x1000) as *const u8)
                } else {
                    /* Illegal to access, mirror WRAM */
                    *((mem_addr - 0x2000) as *const u8)
                }
            }
        }
    };

    [$ss:expr; $a:expr] => (
        mem!($ss; $a)
    );
}

#[macro_export]
#[cfg(not(target_os = "linux"))]
macro_rules! mem {
    [$ss:expr; read $a:expr] => (
        mem!($ss; $a)
    );

    [$ss:expr; write $v:expr => $a:expr] => (
        mem!($ss; $v => $a)
    );

    ($ss:expr; $v:expr => $a:expr) => {
        {
            let system_state: &mut $crate::system_state::SystemState = $ss;
            let gb_addr: u16 = $a;
            let value: u8 = $v;

            unsafe {
                if gb_addr < 0x8000 {
                    /* ROM */
                    system_state.addr_space.rom_write(gb_addr, value);
                } else if gb_addr < 0xe000 {
                    if gb_addr >= 0xc000 {
                        /* Working RAM */
                        system_state.addr_space.wram_write(gb_addr, value);
                    } else if gb_addr < 0xa000 {
                        /* Video RAM */
                        system_state.addr_space.vram_write(gb_addr, value);
                    } else {
                        /* External RAM */
                        system_state.addr_space.extram_write(gb_addr, value);
                    }
                } else if (0xff80..0xffff).contains(&gb_addr) {
                    /* High WRAM and stack */
                    system_state.addr_space.hram_write(gb_addr, value);
                } else if gb_addr >= 0xfea0 {
                    if gb_addr >= 0xff00 {
                        /* I/O */
                        io_write(system_state, gb_addr - 0xff00, value);
                    } else {
                        /* Illegal to access, mirror WRAM */
                        system_state.addr_space.wram_write(gb_addr - 0x2000, value);
                    }
                } else if gb_addr >= 0xfe00 {
                    /* OAM */
                    system_state.addr_space.hram_write(gb_addr, value);
                } else {
                    /* Illegal to access, mirror WRAM */
                    system_state.addr_space.wram_write(gb_addr - 0x2000, value);
                }
            }
        }
    };

    [$ss:expr; $v:expr => $a:expr] => (
        mem!($ss; $v => $a)
    );

    ($ss:expr; $a:expr) => {
        {
            let system_state: &mut $crate::system_state::SystemState = $ss;
            let gb_addr: u16 = $a;

            unsafe {
                if gb_addr < 0x8000 {
                    /* ROM */
                    system_state.addr_space.rom_read(gb_addr)
                } else if gb_addr < 0xe000 {
                    if gb_addr >= 0xc000 {
                        /* Working RAM */
                        system_state.addr_space.wram_read(gb_addr)
                    } else if gb_addr < 0xa000 {
                        /* Video RAM */
                        system_state.addr_space.vram_read(gb_addr)
                    } else {
                        /* External RAM */
                        system_state.addr_space.extram_read(gb_addr)
                    }
                } else if (0xff80..0xffff).contains(&gb_addr) {
                    /* High WRAM and stack */
                    system_state.addr_space.hram_read(gb_addr)
                } else if gb_addr >= 0xfea0 {
                    if gb_addr >= 0xff00 {
                        /* I/O */
                        io_read(system_state, gb_addr - 0xff00)
                    } else {
                        /* Illegal to access, mirror WRAM */
                        system_state.addr_space.wram_read(gb_addr - 0x2000)
                    }
                } else if gb_addr >= 0xfe00 {
                    /* OAM */
                    system_state.addr_space.hram_read(gb_addr)
                } else {
                    /* Illegal to access, mirror WRAM */
                    system_state.addr_space.wram_read(gb_addr - 0x2000)
                }
            }
        }
    };

    [$ss:expr; $a:expr] => (
        mem!($ss; $a)
    );
}

#[macro_export]
macro_rules! regs8 {
    [$cpu:ident.a] => ($cpu.regs8[1]);
    [$cpu:ident.b] => ($cpu.regs8[3]);
    [$cpu:ident.c] => ($cpu.regs8[2]);
    [$cpu:ident.d] => ($cpu.regs8[5]);
    [$cpu:ident.e] => ($cpu.regs8[4]);
    [$cpu:ident.f] => ($cpu.regs8[0]);
    [$cpu:ident.h] => ($cpu.regs8[7]);
    [$cpu:ident.l] => ($cpu.regs8[6]);

    [$v:expr => $cpu:ident.$reg:ident] => (regs8![$cpu.$reg] = $v);
}

#[macro_export]
macro_rules! regs16_split {
    ($cpu:ident, $hi:ident, $lo:ident) => {
        /* Requires little endian */
        unsafe {
            let ptr: *const u8 = &regs8![$cpu.$lo];
            *(ptr as *const u16)
        }
    };

    ($cpu:ident, $hi:ident, $lo:ident, $v:expr) => {
        {
            let value: u16 = $v;
            let ptr: *mut u8 = &mut regs8![$cpu.$lo];

            /* Requires little endian */
            unsafe {
                *(ptr as *mut u16) = value
            }
        }
    };
}

#[macro_export]
macro_rules! regs16 {
    [$cpu:ident.sp] => ($cpu.sp);
    [$cpu:ident.pc] => ($cpu.pc);
    [$cpu:ident.af] => (regs16_split!($cpu, a, f));
    [$cpu:ident.bc] => (regs16_split!($cpu, b, c));
    [$cpu:ident.de] => (regs16_split!($cpu, d, e));
    [$cpu:ident.hl] => (regs16_split!($cpu, h, l));

    [$v:expr => $cpu:ident.sp] => ($cpu.sp = $v);
    [$v:expr => $cpu:ident.pc] => ($cpu.pc = $v);
    [$v:expr => $cpu:ident.af] => (regs16_split!($cpu, a, f, $v));
    [$v:expr => $cpu:ident.bc] => (regs16_split!($cpu, b, c, $v));
    [$v:expr => $cpu:ident.de] => (regs16_split!($cpu, d, e, $v));
    [$v:expr => $cpu:ident.hl] => (regs16_split!($cpu, h, l, $v));
}

#[macro_export]
macro_rules! regs {
    [$cpu:ident.a] => (regs8![$cpu.a]);
    [$cpu:ident.b] => (regs8![$cpu.b]);
    [$cpu:ident.c] => (regs8![$cpu.c]);
    [$cpu:ident.d] => (regs8![$cpu.d]);
    [$cpu:ident.e] => (regs8![$cpu.e]);
    [$cpu:ident.f] => (regs8![$cpu.f]);
    [$cpu:ident.h] => (regs8![$cpu.h]);
    [$cpu:ident.l] => (regs8![$cpu.l]);

    [$cpu:ident.af] => (regs16![$cpu.af]);
    [$cpu:ident.bc] => (regs16![$cpu.bc]);
    [$cpu:ident.de] => (regs16![$cpu.de]);
    [$cpu:ident.hl] => (regs16![$cpu.hl]);
    [$cpu:ident.sp] => (regs16![$cpu.sp]);
    [$cpu:ident.pc] => (regs16![$cpu.pc]);

    [$v:expr => $cpu:ident.a] => (regs8![$v => $cpu.a]);
    [$v:expr => $cpu:ident.b] => (regs8![$v => $cpu.b]);
    [$v:expr => $cpu:ident.c] => (regs8![$v => $cpu.c]);
    [$v:expr => $cpu:ident.d] => (regs8![$v => $cpu.d]);
    [$v:expr => $cpu:ident.e] => (regs8![$v => $cpu.e]);
    [$v:expr => $cpu:ident.f] => (regs8![$v => $cpu.f]);
    [$v:expr => $cpu:ident.h] => (regs8![$v => $cpu.h]);
    [$v:expr => $cpu:ident.l] => (regs8![$v => $cpu.l]);

    [$v:expr => $cpu:ident.af] => (regs16![$v => $cpu.af]);
    [$v:expr => $cpu:ident.bc] => (regs16![$v => $cpu.bc]);
    [$v:expr => $cpu:ident.de] => (regs16![$v => $cpu.de]);
    [$v:expr => $cpu:ident.hl] => (regs16![$v => $cpu.hl]);
    [$v:expr => $cpu:ident.sp] => (regs16![$v => $cpu.sp]);
    [$v:expr => $cpu:ident.pc] => (regs16![$v => $cpu.pc]);
}

#[macro_export]
macro_rules! single_flag_mask {
    (cf) => (0x10u8);
    (hf) => (0x20u8);
    (nf) => (0x40u8);
    (zf) => (0x80u8);
}

#[macro_export]
macro_rules! flags {
    {
        $cpu:ident;
        $($f:ident: $sf:expr),*
    } => {
        let mask = $(single_flag_mask!($f))|* | 0x0fu8;
        regs8![(regs8![$cpu.f] & !mask) |
               $(if $sf { single_flag_mask!($f) } else { 0x00u8 })|*
               => $cpu.f]
    };

    ($cpu:ident.$f:ident) => (regs8![$cpu.f] & single_flag_mask!($f) != 0x00u8);
}
