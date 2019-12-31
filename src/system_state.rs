use crate::address_space::AddressSpace;
use crate::io;
use crate::io::lcd::DisplayState;
use crate::io::timer::TimerState;

#[allow(dead_code)]
pub enum IOReg {
    P1      = 0x00,
    SB      = 0x01,
    SC      = 0x02,
    DIV     = 0x04,
    TIMA    = 0x05,
    TMA     = 0x06,
    TAC     = 0x07,
    IF      = 0x0f,
    NR10    = 0x10,
    NR11    = 0x11,
    NR12    = 0x12,
    NR13    = 0x13,
    NR14    = 0x14,
    NR20    = 0x15,
    NR21    = 0x16,
    NR22    = 0x17,
    NR23    = 0x18,
    NR24    = 0x19,
    NR30    = 0x1a,
    NR31    = 0x1b,
    NR32    = 0x1c,
    NR33    = 0x1d,
    NR34    = 0x1e,
    NR40    = 0x1f,
    NR41    = 0x20,
    NR42    = 0x21,
    NR43    = 0x22,
    NR44    = 0x23,
    NR50    = 0x24,
    NR51    = 0x25,
    NR52    = 0x26,
    WAVE00  = 0x30,
    WAVE02  = 0x31,
    WAVE04  = 0x32,
    WAVE06  = 0x33,
    WAVE08  = 0x34,
    WAVE0a  = 0x35,
    WAVE0c  = 0x36,
    WAVE0e  = 0x37,
    WAVE10  = 0x38,
    WAVE12  = 0x39,
    WAVE14  = 0x3a,
    WAVE16  = 0x3b,
    WAVE18  = 0x3c,
    WAVE1a  = 0x3d,
    WAVE1c  = 0x3e,
    WAVE1e  = 0x3f,
    LCDC    = 0x40,
    STAT    = 0x41,
    SCY     = 0x42,
    SCX     = 0x43,
    LY      = 0x44,
    LYC     = 0x45,
    DMA     = 0x46,
    BGP     = 0x47,
    OBP0    = 0x48,
    OBP1    = 0x49,
    WY      = 0x4a,
    WX      = 0x4b,
    KEY1    = 0x4d,
    VBK     = 0x4f,
    HDMA1   = 0x51,
    HDMA2   = 0x52,
    HDMA3   = 0x53,
    HDMA4   = 0x54,
    HDMA5   = 0x55,
    RP      = 0x56,
    BCPS    = 0x68,
    BCPD    = 0x69,
    OCPS    = 0x6a,
    OCPD    = 0x6b,
    SVBK    = 0x70,
    IE      = 0xff,
}

pub struct SystemState {
    pub addr_space: AddressSpace,

    pub cgb: bool,
    pub ints_enabled: bool,
    pub double_speed: bool,

    pub io_regs: [u8; 256],

    pub display: DisplayState,
    pub timer: TimerState,
}


impl SystemState {
    pub fn new(addr_space: AddressSpace) -> Self {
        let mut state = Self {
            addr_space: addr_space,

            cgb: false,
            ints_enabled: true,
            double_speed: false,

            io_regs: [0u8; 256],

            display: DisplayState::new(),
            timer: TimerState::new(),
        };

        DisplayState::init_system_state(&mut state);

        state.io_regs[IOReg::NR10 as usize] = 0x80;
        state.io_regs[IOReg::NR11 as usize] = 0xBF;
        state.io_regs[IOReg::NR12 as usize] = 0xF3;
        state.io_regs[IOReg::NR14 as usize] = 0xBF;
        state.io_regs[IOReg::NR21 as usize] = 0x3F;
        state.io_regs[IOReg::NR22 as usize] = 0x00;
        state.io_regs[IOReg::NR24 as usize] = 0xBF;
        state.io_regs[IOReg::NR30 as usize] = 0x7F;
        state.io_regs[IOReg::NR31 as usize] = 0xFF;
        state.io_regs[IOReg::NR32 as usize] = 0x9F;
        state.io_regs[IOReg::NR33 as usize] = 0xBF;
        state.io_regs[IOReg::NR41 as usize] = 0xFF;
        state.io_regs[IOReg::NR42 as usize] = 0x00;
        state.io_regs[IOReg::NR43 as usize] = 0x00;
        state.io_regs[IOReg::NR44 as usize] = 0xBF;
        state.io_regs[IOReg::NR50 as usize] = 0x77;
        state.io_regs[IOReg::NR51 as usize] = 0xF3;
        state.io_regs[IOReg::NR52 as usize] = 0xF1;

        // FIXME (shouldn't be here)
        state.io_regs[IOReg::HDMA1 as usize] = 0x00;
        state.io_regs[IOReg::HDMA2 as usize] = 0x00;
        state.io_regs[IOReg::HDMA3 as usize] = 0x80;
        state.io_regs[IOReg::HDMA4 as usize] = 0x00;
        state.io_regs[IOReg::HDMA5 as usize] = 0x80;

        state
    }

    pub fn add_cycles(&mut self, count: u32) {
        io::lcd::add_cycles(self, count);
        io::timer::add_cycles(self, count);
    }
}
