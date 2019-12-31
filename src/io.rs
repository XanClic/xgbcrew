pub mod int;
pub mod ir;
pub mod keypad;
pub mod lcd;
pub mod serial;
pub mod timer;

use crate::address_space::{AS_BASE, get_raw_read_addr};
use crate::system_state::{IOReg, SystemState};


pub fn io_read(sys_state: &mut SystemState, addr: u16) -> u8 {
    sys_state.io_regs[addr as usize]
}

pub fn io_write(sys_state: &mut SystemState, addr: u16, val: u8) {
    assert!(addr < 256);

    IOW_HANDLERS[addr as usize](sys_state, addr, val);
}

fn iow_not_implemented(_: &mut SystemState, addr: u16, val: u8) {
    panic!("I/O register not implemented: 0x{:02x} => 0xff{:02x}", val, addr);
}

fn iow_plain(sys_state: &mut SystemState, addr: u16, val: u8) {
    sys_state.io_regs[addr as usize] = val;
}

fn iow_clear(sys_state: &mut SystemState, addr: u16, _: u8) {
    sys_state.io_regs[addr as usize] = 0u8;
}

fn vbk_write(sys_state: &mut SystemState, _: u16, val: u8) {
    if !sys_state.cgb {
        return;
    }

    sys_state.addr_space.vram_bank = val as usize & 0x01;
    sys_state.addr_space.remap_vram();

    sys_state.io_regs[IOReg::VBK as usize] = val & 0x01;
}

fn svbk_write(sys_state: &mut SystemState, _: u16, val: u8) {
    if !sys_state.cgb {
        return;
    }

    let bank = val as usize & 0x07;
    sys_state.addr_space.wram_bank = if bank == 0 { 1 } else { bank };
    sys_state.addr_space.remap_wramn();

    sys_state.io_regs[IOReg::SVBK as usize] = bank as u8;
}

fn key1_write(sys_state: &mut SystemState, _: u16, val: u8) {
    if !sys_state.cgb {
        return;
    }

    sys_state.io_regs[IOReg::KEY1 as usize] =
        (sys_state.io_regs[IOReg::KEY1 as usize] & 0x80) |
        val & 0x01;
}

fn dma_write(_: &mut SystemState, _: u16, val: u8) {
    if val == 0xff {
        return;
    }

    let src = get_raw_read_addr((val as u16) << 8);

    unsafe {
        libc::memcpy((AS_BASE + 0x10e00) as *mut libc::c_void,
                     src as *const libc::c_void,
                     160);
    }
}

pub fn hdma_copy_16b(sys_state: &mut SystemState) -> bool {
    let hdma = (sys_state.io_regs[IOReg::HDMA1 as usize],
                sys_state.io_regs[IOReg::HDMA2 as usize],
                sys_state.io_regs[IOReg::HDMA3 as usize],
                sys_state.io_regs[IOReg::HDMA4 as usize]);

    let mut src = ((hdma.0 as u16) << 8) | (hdma.1 as u16);
    let mut dst = ((hdma.2 as u16) << 8) | (hdma.3 as u16);

    let raw_src = get_raw_read_addr(src);
    let raw_dst = get_raw_read_addr(dst);

    src += 16;
    dst += 16;

    sys_state.io_regs[IOReg::HDMA1 as usize] = (src >> 8) as u8;
    sys_state.io_regs[IOReg::HDMA2 as usize] = src as u8;
    sys_state.io_regs[IOReg::HDMA3 as usize] = (dst >> 8) as u8;
    sys_state.io_regs[IOReg::HDMA4 as usize] = dst as u8;

    unsafe {
        libc::memcpy(raw_dst as *mut libc::c_void,
                     raw_src as *const libc::c_void,
                     16);
    }

    let (rem, done) =
        sys_state.io_regs[IOReg::HDMA5 as usize].overflowing_sub(1u8);
    sys_state.io_regs[IOReg::HDMA5 as usize] = rem;

    sys_state.add_cycles(if sys_state.double_speed { 16 } else { 8 });

    done
}

fn hdma_write(sys_state: &mut SystemState, addr: u16, mut val: u8) {
    if !sys_state.cgb {
        iow_plain(sys_state, addr, val);
        return;
    }

    match addr {
        0x51 => {
            if val >= 0x80 && val < 0xa0 {
                val = 0;
            } else if val >= 0xe0 {
                val -= 0x20;
            }
        },

        0x53 => {
            val = (val & 0x1f) | 0x80;
        },

        0x52 | 0x54 => (),

        0x55 => {
            if sys_state.io_regs[IOReg::HDMA5 as usize] & 0x80 == 0 {
                /* HDMA active */
                if val & 0x80 == 0 {
                    val = 0xff;
                } else {
                    val = 0x7f;
                }
            } else {
                sys_state.io_regs[IOReg::HDMA5 as usize] = val & 0x7f;

                if val & 0x80 == 0 {
                    while !hdma_copy_16b(sys_state) { }
                }

                return;
            }
        },

        _ => unreachable!(),
    }

    sys_state.io_regs[addr as usize] = val;
}

const IOW_HANDLERS: [fn(&mut SystemState, u16, u8); 256] = [
    keypad::p1_write,                   /* 0x00 */
    serial::serial_write,
    serial::serial_write,
    iow_not_implemented,
    iow_clear, /* DIV */
    timer::timer_write,
    timer::timer_write,
    timer::timer_write,
    iow_not_implemented,                /* 0x08 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_plain, /* interrupt flags */
    iow_plain, /* TODO: Sound */        /* 0x10 */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */        /* 0x18 */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */        /* 0x20 */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_not_implemented,
    iow_not_implemented,                /* 0x28 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_plain, /* TODO: Sound */        /* 0x30 */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */        /* 0x38 */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    iow_plain, /* TODO: Sound */
    lcd::lcd_write,                     /* 0x40 */
    lcd::lcd_write,
    lcd::lcd_write,
    lcd::lcd_write,
    lcd::lcd_write,
    lcd::lcd_write,
    dma_write,
    lcd::lcd_write,
    lcd::lcd_write,                     /* 0x48 */
    lcd::lcd_write,
    lcd::lcd_write,
    lcd::lcd_write,
    iow_not_implemented,
    key1_write,
    iow_not_implemented,
    vbk_write,
    iow_not_implemented,                /* 0x50 */
    hdma_write,
    hdma_write,
    hdma_write,
    hdma_write,
    hdma_write,
    ir::rp_write,
    iow_not_implemented,
    iow_not_implemented,                /* 0x58 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0x60 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    lcd::lcd_write,                     /* 0x68 */
    lcd::lcd_write,
    lcd::lcd_write,
    lcd::lcd_write,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    svbk_write,                         /* 0x70 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0x78 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0x80 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0x88 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0x90 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0x98 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xa0 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xa8 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xb0 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xb8 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xc0 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xc8 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xd0 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xd8 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xe0 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xe8 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xf0 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,                /* 0xf8 */
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_not_implemented,
    iow_plain, /* interrupt enable */
];
