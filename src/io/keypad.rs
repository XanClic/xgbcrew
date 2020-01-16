use crate::address_space::AddressSpace;
use crate::io::IOSpace;
use crate::io::int::IRQ;
use crate::sgb::sgb_pulse;
use crate::system_state::{IOReg, SystemState};


#[derive(SaveState)]
pub struct KeypadState {
    /* Do not export keyboard state, because reloading does not change
     * what the user is pressing */
    #[savestate(skip)]
    all_lines: u8,

    mask: u8,

    #[savestate(skip_if("version < 1"))]
    sgb_cooldown: bool,

    #[savestate(skip_if("version < 1"))]
    controller_count: usize, /* Used by SGB */

    #[savestate(skip_if("version < 1"))]
    controller_index: usize, /* Used by SGB */
}

pub enum KeypadKey {
    A,
    B,
    Start,
    Select,

    Left,
    Right,
    Up,
    Down,
}

impl KeypadState {
    pub fn new() -> Self {
        Self {
            all_lines: 0,
            mask: 0,

            sgb_cooldown: false,
            controller_count: 1,
            controller_index: 0,
        }
    }

    pub fn init_system_state(sys_state: &mut SystemState) {
        sys_state.keypad.update_p1(&mut sys_state.addr_space);
    }

    fn update_p1(&self, addr_space: &mut AddressSpace) {
        let p14_15 =
            (if self.mask & 0x0f == 0x00 { 0x10 } else { 0x00 }) |
            (if self.mask & 0xf0 == 0x00 { 0x20 } else { 0x00 });

        let nibbles =
            if self.controller_index == 0 {
                ((self.all_lines & self.mask) & 0xf,
                 (self.all_lines & self.mask) >> 4)
            } else {
                (0, 0)
            };

        if self.mask == 0 {
            let ci = self.controller_index as u8;
            addr_space.io_set_reg(IOReg::P1, p14_15 | (0xf - ci));
        } else {
            addr_space.io_set_reg(IOReg::P1, p14_15 | !(nibbles.0 | nibbles.1));
        }
    }

    pub fn post_import(&self, addr_space: &mut AddressSpace) {
        self.update_p1(addr_space);
    }

    pub fn key_event(&mut self, addr_space: &mut AddressSpace,
                     key: KeypadKey, down: bool)
    {
        let line =
            match key {
                KeypadKey::Right    => (1 << 0),
                KeypadKey::Left     => (1 << 1),
                KeypadKey::Up       => (1 << 2),
                KeypadKey::Down     => (1 << 3),

                KeypadKey::A        => (1 << 4),
                KeypadKey::B        => (1 << 5),
                KeypadKey::Select   => (1 << 6),
                KeypadKey::Start    => (1 << 7),
            };

        if down {
            self.all_lines |= line;

            if line & self.mask != 0 {
                let iflag = addr_space.io_get_reg(IOReg::IF);
                addr_space.io_set_reg(IOReg::IF, iflag | (IRQ::Input as u8));
            }
        } else {
            self.all_lines &= !line;
        }

        self.update_p1(addr_space);
    }

    pub fn set_controller_count(&mut self, count: usize) {
        self.controller_count = count;
        self.controller_index = count - 1;
    }
}


pub fn p1_write(sys_state: &mut SystemState, _: u16, val: u8)
{
    let kp = &mut sys_state.keypad;

    let np14 = val & 0x10 == 0;
    let np15 = val & 0x20 == 0;

    kp.mask =
        (if np14 { 0x0f } else { 0x00 }) |
        (if np15 { 0xf0 } else { 0x00 });

    if !np14 && !np15 && kp.controller_count > 0 {
        kp.controller_index += 1;
        kp.controller_index %= kp.controller_count;
    }

    kp.update_p1(&mut sys_state.addr_space);

    if sys_state.sgb {
        if !kp.sgb_cooldown && (np14 || np15) {
            kp.sgb_cooldown = true;
            sgb_pulse(sys_state, np14, np15);
        } else if kp.sgb_cooldown && !(np14 || np15) {
            kp.sgb_cooldown = false;
        }
    }
}
