use sdl2::keyboard::Scancode;

use crate::io::{io_get_reg, io_set_reg};
use crate::io::int::IRQ;
use crate::system_state::{IOReg, SystemState};


pub struct KeypadState {
    all_lines: u8,
    mask: u8,
}

impl KeypadState {
    pub fn new() -> Self {
        Self {
            all_lines: 0,
            mask: 0,
        }
    }

    pub fn init_system_state(sys_state: &mut SystemState) {
        sys_state.keypad.update_p1();
    }

    fn update_p1(&self) {
        let p14_15 =
            (if self.mask & 0x0f == 0x00 { 0x10 } else { 0x00 }) |
            (if self.mask & 0xf0 == 0x00 { 0x20 } else { 0x00 });

        let nibbles = ((self.all_lines & self.mask) & 0xf,
                       (self.all_lines & self.mask) >> 4);

        io_set_reg(IOReg::P1, p14_15 | !(nibbles.0 | nibbles.1));
    }

    fn key_event(&mut self, key: Scancode, down: bool) {
        let line =
            match key {
                Scancode::Right     => (1 << 0),
                Scancode::Left      => (1 << 1),
                Scancode::Up        => (1 << 2),
                Scancode::Down      => (1 << 3),

                Scancode::X         => (1 << 4),
                Scancode::Z         => (1 << 5),
                Scancode::Backspace => (1 << 6),
                Scancode::Return    => (1 << 7),

                _ => return
            };

        if down {
            self.all_lines |= line;

            if line & self.mask != 0 {
                io_set_reg(IOReg::IF,
                           io_get_reg(IOReg::IF) | (IRQ::Input as u8));
            }
        } else {
            self.all_lines &= !line;
        }

        self.update_p1();
    }
}


pub fn p1_write(sys_state: &mut SystemState, _: u16, val: u8)
{
    let kp = &mut sys_state.keypad;

    kp.mask =
        (if val & 0x10 == 0 { 0x0f } else { 0x00 }) |
        (if val & 0x20 == 0 { 0xf0 } else { 0x00 });

    kp.update_p1();
}


pub fn key_event(sys_state: &mut SystemState, key: Scancode, down: bool) {
    if key == Scancode::Space {
        sys_state.realtime = !down;
    } else {
        sys_state.keypad.key_event(key, down);
    }
}
