use crate::io::io_set_reg;
use crate::system_state::{IOReg, SystemState};


pub fn rp_write(_: &mut SystemState, _: u16, val: u8)
{
    /* TODO */
    io_set_reg(IOReg::RP, val & 0xc1);
}
