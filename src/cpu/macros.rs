#[macro_export]
macro_rules! mem {
    [$ss:expr; read $t:ty:$a:expr] => (
        mem!($ss; $t; $a)
    );

    [$ss:expr; write $v:expr => $t:ty:$a:expr] => (
        mem!($ss; $t; $v => $a)
    );

    ($ss:expr; $t:ty; $v:expr => $a:expr) => {
        unsafe {
            let mem_addr = AS_BASE + ($a as usize);

            if $a < 0x8000u16 {
                /* ROM */
                let val = { $v };
                let rom_write = |addr, x| $ss.addr_space.rom_write(addr, x);

                val.split_into_u8($a, rom_write);
            } else if $a < 0xe000u16 {
                if $a < 0xa000u16 || $a >= 0xc000 {
                    /* Video/Working RAM */
                    *(mem_addr as *mut $t) = $v;
                } else {
                    /* External RAM */
                    if $ss.addr_space.extram_rw {
                        *(mem_addr as *mut $t) = $v;
                    } else {
                        let val = { $v };
                        let extram_write = |addr, x|
                            $ss.addr_space.extram_write(addr, x);

                        val.split_into_u8($a, extram_write);
                    }
                }
            } else if $a >= 0xff80u16 && $a < 0xffffu16 {
                /* High WRAM and stack */
                *((mem_addr + 0x1000) as *mut $t) = $v;
            } else if $a >= 0xfea0u16 {
                if $a >= 0xff00u16 {
                    /* I/O */
                    let val = { $v };
                    let iow = |addr, x| io_write($ss, addr, x);

                    val.split_into_u8($a - 0xff00, iow);
                } else {
                    /* Illegal to access, mirror WRAM */
                    *((mem_addr - 0x2000) as *mut $t) = $v;
                }
            } else if $a >= 0xfe00u16 {
                /* OAM */
                *((mem_addr + 0x1000) as *mut $t) = $v;
            } else {
                /* Illegal to access, mirror WRAM */
                *((mem_addr - 0x2000) as *mut $t) = $v;
            }
        }
    };

    [$ss:expr; $v:expr => $t:ty:$a:expr] => (
        mem!($ss; $t; $v => $a)
    );

    ($ss:expr; $t:ty; $a:expr) => {
        unsafe {
            let mem_addr = AS_BASE + ($a as usize);

            if $a < 0xe000u16 {
                /* Normal AS */
                *(mem_addr as *mut $t)
            } else if $a >= 0xff80u16 && $a < 0xffffu16 {
                /* High WRAM and stack */
                *((mem_addr + 0x1000) as *mut $t)
            } else if $a >= 0xfea0u16 {
                if $a >= 0xff00u16 {
                    /* I/O */
                    let ior = |addr| io_read($ss, addr);
                    <$t>::construct_from_u8($a - 0xff00, ior)
                } else {
                    /* Illegal to access, mirror WRAM */
                    *((mem_addr - 0x2000) as *mut $t)
                }
            } else if $a >= 0xfe00u16 {
                /* OAM */
                *((mem_addr + 0x1000) as *mut $t)
            } else {
                /* Illegal to access, mirror WRAM */
                *((mem_addr - 0x2000) as *mut $t)
            }
        }
    };

    [$ss:expr; $t:ty:$a:expr] => (
        mem!($ss; $t; $a)
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
        /* Requires little endian */
        unsafe {
            let ptr: *mut u8 = &mut regs8![$cpu.$lo];
            *(ptr as *mut u16) = $v
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
