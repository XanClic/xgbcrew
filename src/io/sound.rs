pub mod functionality;

use std::cmp;
use functionality::GlobalAudioState;
pub use functionality::SoundState;

use crate::address_space::AddressSpace;
use crate::io::IOSpace;
use crate::system_state::{IOReg, SystemState};


impl GlobalAudioState for AddressSpace {
    fn enable_channel(&mut self, channel: usize, enabled: bool) {
        let nr52 = self.io_get_reg(IOReg::NR52);
        let nr52 = if enabled {
            nr52 | (1 << channel)
        } else {
            nr52 & !(1 << channel)
        };
        self.io_set_reg(IOReg::NR52, nr52);
    }

    fn wave_sample(&self, i: usize) -> u8 {
        self.io_get_addr(0x30 + i as u16)
    }
}

fn reset_regs(addr_space: &mut AddressSpace) {
    addr_space.io_set_reg(IOReg::NR10, 0x80);
    addr_space.io_set_reg(IOReg::NR11, 0xbf);
    addr_space.io_set_reg(IOReg::NR12, 0xf3);
    addr_space.io_set_reg(IOReg::NR14, 0xbf);
    addr_space.io_set_reg(IOReg::NR21, 0x3f);
    addr_space.io_set_reg(IOReg::NR22, 0x00);
    addr_space.io_set_reg(IOReg::NR24, 0xbf);
    addr_space.io_set_reg(IOReg::NR30, 0x7f);
    addr_space.io_set_reg(IOReg::NR31, 0xff);
    addr_space.io_set_reg(IOReg::NR32, 0x9f);
    addr_space.io_set_reg(IOReg::NR33, 0xbf);
    addr_space.io_set_reg(IOReg::NR41, 0xff);
    addr_space.io_set_reg(IOReg::NR42, 0x00);
    addr_space.io_set_reg(IOReg::NR43, 0x00);
    addr_space.io_set_reg(IOReg::NR44, 0xbf);
    addr_space.io_set_reg(IOReg::NR50, 0x77);
    addr_space.io_set_reg(IOReg::NR51, 0xf3);
    addr_space.io_set_reg(IOReg::NR52, 0xf1);
}

pub fn sound_write(sys_state: &mut SystemState, addr: u16, mut val: u8)
{
    let s = &mut sys_state.sound;
    let addr_space = sys_state.addr_space.as_mut();
    let nr52 = addr_space.io_get_reg(IOReg::NR52);

    if nr52 & 0x80 == 0 && addr != 0x26 {
        return;
    }

    match addr {
        0x10 => {
            s.ch1.set_nrx0(val);
            val &= 0x7f;
        },

        0x11 => {
            s.ch1.set_nrx1(val);
            val &= 0xc0;
        },

        0x12 => {
            s.ch1.set_nrx2(val);
        },

        0x13 => {
            s.ch1.set_nrx3(val);
            val = 0;
        },

        0x14 => {
            s.ch1.set_nrx4(val, addr_space);
            val &= 0x40;
        },

        0x15 => {
            val = 0;
        },

        0x16 => {
            s.ch2.set_nrx1(val);
            val &= 0xc0;
        },

        0x17 => {
            s.ch2.set_nrx2(val);
        },

        0x18 => {
            s.ch2.set_nrx3(val);
            val = 0;
        },

        0x19 => {
            s.ch2.set_nrx4(val, addr_space);
            val &= 0x40;
        },

        0x1a => {
            s.ch3.set_nrx0(val, addr_space);
            val &= 0x80;
        },

        0x1b => {
            s.ch3.set_nrx1(val);
        },

        0x1c => {
            s.ch3.set_nrx2(val);
            val &= 0x60;
        },

        0x1d => {
            s.ch3.set_nrx3(val);
            val = 0;
        },

        0x1e => {
            s.ch3.set_nrx4(val, addr_space);
            val &= 0x40;
        },

        0x1f => {
            val = 0;
        },

        0x20 => {
            s.ch4.set_nrx1(val);
            val = 0;
        },

        0x21 => {
            s.ch4.set_nrx2(val);
        },

        0x22 => {
            s.ch4.set_nrx3(val);
        },

        0x23 => {
            s.ch4.set_nrx4(val, addr_space);
            val &= 0x40;
        },

        0x24 => {
            // Never mute
            s.shared.lvol = (cmp::max((val >> 4) & 0x07, 1)) as f32;
            s.shared.rvol = (cmp::max(val & 0x07, 1)) as f32;
        },

        0x25 => {
            s.shared.channel_mask = val;
        },

        0x26 => {
            val = (val & 0x80) | (nr52 & 0xf);
            if val & 0x80 == 0 {
                reset_regs(addr_space);
                s.reset();
            }
        },

        0x30..=0x3f => (),

        _ => unreachable!(),
    }

    addr_space.io_set_addr(addr, val);
}
