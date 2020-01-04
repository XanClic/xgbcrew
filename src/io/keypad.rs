use crate::io::io_set_reg;
use crate::system_state::{IOReg, SystemState};


pub fn p1_write(_: &mut SystemState, _: u16, val: u8)
{
    /* TODO: Read key state */
    io_set_reg(IOReg::P1, (val & 0x30) | 0x0f);
}
