mod insns;
#[macro_use] mod macros;

use crate::io::IOSpace;
use crate::system_state::{IOReg, SystemState};


#[derive(Serialize, Deserialize, Clone)]
pub enum IIOperation {
    EnableInterrupts,
    DisableInterrupts,
}

#[derive(Serialize, Deserialize, Clone)]
struct InternalInstruction {
    delay: i8,
    op: IIOperation,
}

#[derive(SaveState)]
pub struct CPU {
    /* Order here: f, a, c, b, e, d, l, h */
    /* (Indices used in CPU instructions: b, c, d, e, h, l, (none), a) */
    regs8: [u8; 8],
    sp: u16,
    pc: u16,

    halted: bool,

    /* Should generally be small enough that a vec is best */
    internal_insns: Vec<InternalInstruction>,
}

impl CPU {
    pub fn new(cgb: bool, sgb: bool) -> Self {
        let (a, f, b, c, d, e, h, l) =
            if cgb {
                (0x11u8, 0xb0u8, 0x00u8, 0x00u8,
                 0xffu8, 0x56u8, 0x00u8, 0x0du8)
            } else if sgb {
                (0xffu8, 0xb0u8, 0x00u8, 0x13u8,
                 0x00u8, 0xd8u8, 0x01u8, 0x4du8)
            } else {
                (0x01u8, 0xb0u8, 0x00u8, 0x13u8,
                 0x00u8, 0xd8u8, 0x01u8, 0x4du8)
            };

        Self {
            regs8: [f, a, c, b, e, d, l, h],
            sp: 0xfffeu16,
            pc: 0x0100u16,

            halted: false,

            internal_insns: Vec::<InternalInstruction>::new(),
        }
    }

    pub fn exec(&mut self, sys_state: &mut SystemState) -> u32 {
        let cycles =
            if self.halted {
                if !sys_state.ints_enabled {
                    self.halted = false;
                }

                1
            } else {
                if !self.internal_insns.is_empty() {
                    let mut exec_insns = Vec::<IIOperation>::new();

                    for ref mut ii in &mut self.internal_insns {
                        if ii.delay == 0 {
                            exec_insns.push(ii.op.clone());
                        }
                        ii.delay -= 1;
                    }

                    if !exec_insns.is_empty() {
                        self.internal_insns.retain(|ref x| x.delay >= 0);
                        for ii in exec_insns {
                            self.exec_int_insn(sys_state, &ii);
                        }
                    }
                }

                insns::exec(self, sys_state)
            };

        let (ime, irqs) = {
            (sys_state.ints_enabled,
             sys_state.io_get_reg(IOReg::IF) & sys_state.io_get_reg(IOReg::IE))
        };

        if ime {
            if irqs != 0 {
                let irq = irqs.trailing_zeros() as u16;

                sys_state.ints_enabled = false;
                { insns::push(self, sys_state, self.pc); }
                self.pc = 0x40 + irq * 8;

                let iflag = sys_state.io_get_reg(IOReg::IF);
                sys_state.io_set_reg(IOReg::IF, iflag & !(1 << irq));
            }
        }

        cycles
    }

    fn exec_int_insn(&mut self, sys_state: &mut SystemState, ii: &IIOperation) {
        match ii {
            IIOperation::EnableInterrupts => {
                sys_state.ints_enabled = true;
            },

            IIOperation::DisableInterrupts => {
                sys_state.ints_enabled = false;
            },
        }
    }

    fn inject_int_insn(&mut self, delay: i8, op: IIOperation) {
        self.internal_insns.push(InternalInstruction {
            delay: delay,
            op: op,
        });
    }
}
