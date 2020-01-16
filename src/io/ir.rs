use crate::io::IOSpace;
use crate::system_state::{IOReg, SystemState};


pub fn rp_write(sys_state: &mut SystemState, _: u16, val: u8)
{
    /* TODO */
    sys_state.io_set_reg(IOReg::RP, val & 0xc1);
}
