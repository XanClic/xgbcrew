use crate::io::IOSpace;
use crate::system_state::{IOReg, SystemState};


pub fn serial_write(sys_state: &mut SystemState, addr: u16, val: u8)
{
    /* TODO */

    match addr {
        0x01 => {
            /* Discard written data */
        },

        0x02 => {
            /* Pretend immediate end of transfer */
            sys_state.io_set_reg(IOReg::SC, val & 0x01);
        }

        _ => {
            panic!("Unknown serial register 0xff{:02x} (w 0x{:02x})",
                   addr, val);
        }
    }
}
