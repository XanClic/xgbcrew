mod insns;
#[macro_use] mod macros;

use std::cell::RefCell;
use std::rc::Rc;

use crate::system_state::{IOReg, SystemState};


#[derive(Clone)]
pub enum IIOperation {
    EnableInterrupts,
    DisableInterrupts,
}

struct InternalInstruction {
    delay: i8,
    op: IIOperation,
}

pub struct CPU {
    /* f, a, c, b, e, d, l, h */
    /* b, c, d, e, h, l, (none), a */
    regs8: [u8; 8],
    sp: u16,
    pc: u16,

    halted: bool,

    /* Should generally be small enough that a vec is best */
    internal_insns: Vec<InternalInstruction>,

    sys_state: Rc<RefCell<SystemState>>,
}

impl CPU {
    pub fn new(state: Rc<RefCell<SystemState>>)
        -> Self
    {
        if state.borrow().cgb {
            Self {
                regs8: [0xb0u8, 0x11u8, 0x00u8, 0x00u8,
                        0x56u8, 0xffu8, 0x0du8, 0x00u8],
                sp: 0xfffeu16,
                pc: 0x0100u16,

                halted: false,

                internal_insns: Vec::<InternalInstruction>::new(),

                sys_state: state,
            }
        } else {
            Self {
                regs8: [0xb0u8, 0x01u8, 0x13u8, 0x00u8,
                        0xd8u8, 0x00u8, 0x4du8, 0x01u8],
                sp: 0xfffeu16,
                pc: 0x0100u16,

                halted: false,

                internal_insns: Vec::<InternalInstruction>::new(),

                sys_state: state,
            }
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.halted {
                self.add_cycles(1);

                if !self.sys_state.borrow().ints_enabled {
                    self.halted = false;
                }
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
                            self.exec_int_insn(&ii);
                        }
                    }
                }

                insns::exec(self);
            }

            {
                let (ime, irqs) = {
                    let ss = self.sys_state.borrow();

                    (ss.ints_enabled,
                     ss.io_regs[IOReg::IF as usize] &
                     ss.io_regs[IOReg::IE as usize])
                };

                if ime {
                    if irqs != 0 {
                        let irq = irqs.trailing_zeros() as u16;

                        { self.sys_state.borrow_mut().ints_enabled = false; }
                        { insns::push(self, self.pc); }
                        self.pc = 0x40 + irq * 8;

                        let mut ss = self.sys_state.borrow_mut();
                        ss.io_regs[IOReg::IF as usize] &= !(1 << irq);
                    }
                }
            }
        }
    }

    fn exec_int_insn(&mut self, ii: &IIOperation) {
        match ii {
            IIOperation::EnableInterrupts => {
                self.sys_state.borrow_mut().ints_enabled = true;
            },

            IIOperation::DisableInterrupts => {
                self.sys_state.borrow_mut().ints_enabled = false;
            },
        }
    }

    fn inject_int_insn(&mut self, delay: i8, op: IIOperation) {
        self.internal_insns.push(InternalInstruction {
            delay: delay,
            op: op,
        });
    }

    pub fn add_cycles(&mut self, count: u32) {
        self.sys_state.borrow_mut().add_cycles(count);
    }
}
