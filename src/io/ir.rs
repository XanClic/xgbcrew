use crate::system_state::{IOReg, SystemState};


pub fn rp_write(sys_state: &mut SystemState, _: u16, val: u8)
{
    /* TODO */
    sys_state.io_regs[IOReg::RP as usize] = val & 0xc1;
}
