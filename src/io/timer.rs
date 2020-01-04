use crate::io::{io_get_reg, io_set_addr, io_set_reg};
use crate::io::int::IRQ;
use crate::system_state::{IOReg, SystemState};


pub struct TimerState {
    div_counter: u32,

    timer_counter: u32,
    timer_enabled: bool,
    timer_divider: u32,
}


impl TimerState {
    pub fn new() -> Self {
        Self {
            div_counter: 0,

            timer_counter: 0,
            timer_enabled: false,
            timer_divider: 256,
        }
    }
}

pub fn add_cycles(sys_state: &mut SystemState, count: u32) {
    let timer = &mut sys_state.timer;

    timer.div_counter += count;
    while timer.div_counter >= 64 {
        let cur = io_get_reg(IOReg::DIV);
        io_set_reg(IOReg::DIV, cur.wrapping_add(1u8));

        timer.div_counter -= 64;
    }

    if timer.timer_enabled {
        timer.timer_counter += count;
        while timer.timer_counter >= timer.timer_divider {
            let cur = io_get_reg(IOReg::TIMA);
            let (mut res, overflow) = cur.overflowing_add(1u8);

            if overflow {
                io_set_reg(IOReg::IF,
                           io_get_reg(IOReg::IF) | (IRQ::Timer as u8));

                res = io_get_reg(IOReg::TMA);
            }

            io_set_reg(IOReg::TIMA, res);
            timer.timer_counter -= timer.timer_divider;
        }
    }
}

pub fn timer_write(sys_state: &mut SystemState, addr: u16, mut val: u8)
{
    let timer = &mut sys_state.timer;

    if addr == 0x07 {
        /* TAC */
        val &= 0x7;

        timer.timer_enabled = val & (1 << 2) != 0;
        timer.timer_divider = match val & 0x3 {
            0 => 256,
            1 =>   4,
            2 =>  16,
            3 =>  64,

            _ => unreachable!(),
        };
    }

    io_set_addr(addr, val);
}
