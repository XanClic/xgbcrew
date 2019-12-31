use crate::system_state::{IOReg, SystemState};


pub fn p1_write(sys_state: &mut SystemState, _: u16, val: u8)
{
    /* TODO: Read key state */
    sys_state.io_regs[IOReg::P1 as usize] = (val & 0x30) | 0x0f;
}
