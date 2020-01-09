#![allow(non_snake_case)]
#![allow(unused_unsafe)]

use crate::{mem, regs, regs8, regs16, regs16_split, flags, single_flag_mask};
use crate::address_space::{AS_BASE, U8Split};
use crate::cpu::{CPU, IIOperation};
use crate::io::{io_get_reg, io_read, io_set_reg, io_write};
use crate::system_state::{IOReg, SystemState};


/* Maps the opcode-encoded register to the index in CPU.regs8 */
const REG_MAPPING: [usize; 8] = [3, 2, 5, 4, 7, 6, !0usize, 1];


pub fn cpu_panic(cpu: &CPU, msg: &str) {
    panic!("{}\naf={:04x} bc={:04x} de={:04x} hl={:04x}\npc={:04x} sp={:04x}",
           msg,
           regs![cpu.af], regs![cpu.bc], regs![cpu.de], regs![cpu.hl],
           regs![cpu.pc], regs![cpu.sp]);
}

#[allow(dead_code)]
pub fn cpu_debug(cpu: &CPU, sys_state: &SystemState, msg: &str) {
    println!("{}PC={:04x} AF={:04x} BC={:04x} DE={:04x} HL={:04x} SP={:04x} [{}]",
             msg,
             regs![cpu.pc], regs![cpu.af], regs![cpu.bc], regs![cpu.de],
             regs![cpu.hl], regs![cpu.sp],
             sys_state.addr_space.rom_bank);
}


fn n8(cpu: &mut CPU, sys_state: &mut SystemState) -> u8 {
    let val = mem![sys_state; u8:regs![cpu.pc]];
    regs![regs![cpu.pc].wrapping_add(1u16) => cpu.pc];
    val
}

fn n16(cpu: &mut CPU, sys_state: &mut SystemState) -> u16 {
    let val = mem![sys_state; u16:regs![cpu.pc]];
    regs![regs![cpu.pc].wrapping_add(2u16) => cpu.pc];
    val
}


pub fn exec(cpu: &mut CPU, sys_state: &mut SystemState) -> u32 {
    //cpu_debug(cpu, sys_state, "");
    let basic_opcode = n8(cpu, sys_state) as usize;

    if basic_opcode == 0xcb {
        prefix0xcb(cpu, sys_state)
    } else {
        INSN_HANDLERS[basic_opcode](cpu, sys_state);
        INSN_CYCLES[basic_opcode] as u32
    }
}

fn prefix0x10(cpu: &mut CPU, sys_state: &mut SystemState) {
    let sub_op = n8(cpu, sys_state);

    if sub_op != 0x00u8 {
        cpu_panic(cpu,
                  format!("Unknown opcode 0x10 0x{:02x}", sub_op).as_str());
    }

    /* STOP */
    if io_get_reg(IOReg::KEY1) & 0x01 == 0 {
        cpu_panic(cpu, "STOP");
    }

    sys_state.double_speed = !sys_state.double_speed;

    io_set_reg(IOReg::KEY1,
               (io_get_reg(IOReg::KEY1) & !0x80) |
               (if sys_state.double_speed { 0x80 } else { 0x00 }));

    println!("Using {} speed",
             if sys_state.double_speed { "double" } else { "single" });
}

macro_rules! acc_op_r8 {
    [$cpu:ident/$ss:ident.$r:expr] => (
        if $r != 6 {
            $cpu.regs8[REG_MAPPING[$r]]
        } else {
            mem![$ss; u8:regs![$cpu.hl]]
        }
    );

    [$v:expr => $cpu:ident/$ss:ident.$r:expr] => (
        if $r != 6 {
            $cpu.regs8[REG_MAPPING[$r]] = $v
        } else {
            mem![$ss; $v => u8:regs![$cpu.hl]]
        }
    );
}

fn prefix0xcb(cpu: &mut CPU, sys_state: &mut SystemState) -> u32 {
    let prefixed_opcode = n8(cpu, sys_state) as usize;

    if prefixed_opcode < 0x40 {
        INSN_CB_HANDLERS[prefixed_opcode](cpu, sys_state);
    } else {
        let r = prefixed_opcode & 0x07;
        let mask = 1u8 << ((prefixed_opcode & 0x38) >> 3);

        match prefixed_opcode & 0xc0 {
            0x40 /* BIT */ => {
                let rv = acc_op_r8![cpu/sys_state.r];

                flags! { cpu;
                    zf: rv & mask == 0,
                    nf: false,
                    hf: true
                };
            },

            0x80 /* RES */ => {
                let res = acc_op_r8![cpu/sys_state.r] & !mask;
                acc_op_r8![res => cpu/sys_state.r];
            },

            0xc0 /* SET */ => {
                let res = acc_op_r8![cpu/sys_state.r] | mask;
                acc_op_r8![res => cpu/sys_state.r];
            },

            _ => unreachable!(),
        };
    }

    INSN_CB_CYCLES[prefixed_opcode] as u32
}


macro_rules! quasi_r8s {
    ($cpu:ident/$sys_state:ident, n8) => (
        n8($cpu, $sys_state)
    );

    ($cpu:ident/$sys_state:ident, _bc) => (
        mem![$sys_state; u8:regs![$cpu.bc]]
    );

    ($cpu:ident/$sys_state:ident, _de) => (
        mem![$sys_state; u8:regs![$cpu.de]]
    );

    ($cpu:ident/$sys_state:ident, _hl) => (
        mem![$sys_state; u8:regs![$cpu.hl]]
    );

    ($cpu:ident/$sys_state:ident, _ffn8) => ({
        let n = n8($cpu, $sys_state);
        mem![$sys_state; read u8:0xff00u16 + (n as u16)]
    });

    ($cpu:ident/$sys_state:ident, _ffc) => (
        mem![$sys_state; read u8:0xff00u16 + (regs![$cpu.c] as u16)]
    );

    ($cpu:ident/$sys_state:ident, _n16) => ({
        let n = n16($cpu, $sys_state);
        mem![$sys_state; u8:n]
    });

    ($cpu:ident/$sys_state:ident, $r:ident) => (
        regs![$cpu.$r]
    );
}

macro_rules! quasi_r8d {
    ($cpu:ident/$sys_state:ident, _ffn8, $v:expr) => ({
        let n = n8($cpu, $sys_state);
        mem![$sys_state; $v => u8:(0xff00u16 + (n as u16))]
    });

    ($cpu:ident/$sys_state:ident, _bc, $v:expr) => (
        mem![$sys_state; $v => u8:regs![$cpu.bc]]
    );

    ($cpu:ident/$sys_state:ident, _de, $v:expr) => (
        mem![$sys_state; $v => u8:regs![$cpu.de]]
    );

    ($cpu:ident/$sys_state:ident, _hl, $v:expr) => (
        mem![$sys_state; $v => u8:regs![$cpu.hl]]
    );

    ($cpu:ident/$sys_state:ident, _ffc, $v:expr) => (
        mem![$sys_state; $v => u8:(0xff00u16 + (regs![$cpu.c] as u16))]
    );

    ($cpu:ident/$sys_state:ident, _n16, $v:expr) => ({
        let n = n16($cpu, $sys_state);
        mem![$sys_state; $v => u8:n]
    });

    ($cpu:ident/$sys_state:ident, $r:ident, $v:expr) => (
        regs![$v => $cpu.$r]
    );
}

macro_rules! ld_r8_r8 {
    ($rd:ident, $rs:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<ld_ $rd _ $rs>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let val = quasi_r8s!(cpu/sys_state, $rs);
                quasi_r8d!(cpu/sys_state, $rd, val);
            }
        }
    };
}

macro_rules! ld_r16_n16 {
    ($r:ident) => {
        paste::item! {
            fn [<ld_ $r _n16>](cpu: &mut CPU, sys_state: &mut SystemState) {
                regs![n16(cpu, sys_state) => cpu.$r];
            }
        }
    };
}

macro_rules! inc_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<inc_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let res = quasi_r8s!(cpu/sys_state, $r).wrapping_add(1u8);
                quasi_r8d!(cpu/sys_state, $r, res);

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: res & 0x0f == 0
                };
            }
        }
    };
}

macro_rules! dec_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<dec_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let res = quasi_r8s!(cpu/sys_state, $r).wrapping_sub(1u8);
                quasi_r8d!(cpu/sys_state, $r, res);

                flags! { cpu;
                    zf: res == 0,
                    nf: true,
                    hf: res & 0x0f == 0xf
                };
            }
        }
    };
}

macro_rules! add_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<add_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let a = regs![cpu.a];
                let r8 = quasi_r8s!(cpu/sys_state, $r);
                let (res, cf) = a.overflowing_add(r8);

                regs![res => cpu.a];

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: (a ^ r8 ^ res) & 0x10 != 0,
                    cf: cf
                };
            }
        }
    };
}

macro_rules! adc_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<adc_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let a = regs![cpu.a];
                let r8 = quasi_r8s!(cpu/sys_state, $r);
                let res = (a as u32) + (r8 as u32) + (flags![cpu.cf] as u32);
                let res8 = res as u8;

                regs![res8 => cpu.a];

                flags! { cpu;
                    zf: res8 == 0,
                    nf: false,
                    hf: (a ^ r8 ^ res8) & 0x10 != 0,
                    cf: res > 0xff
                };
            }
        }
    };
}

macro_rules! sub_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<sub_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let a = regs![cpu.a];
                let r8 = quasi_r8s!(cpu/sys_state, $r);
                let (res, cf) = a.overflowing_sub(r8);

                regs![res => cpu.a];

                flags! { cpu;
                    zf: res == 0,
                    nf: true,
                    hf: (a ^ r8 ^ res) & 0x10 != 0,
                    cf: cf
                };
            }
        }
    };
}

macro_rules! sbc_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<sbc_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let a = regs![cpu.a];
                let r8 = quasi_r8s!(cpu/sys_state, $r);
                let res = (a as u32).wrapping_sub((r8 as u32) +
                                                  (flags![cpu.cf] as u32));
                let res8 = res as u8;

                regs![res8 => cpu.a];

                flags! { cpu;
                    zf: res8 == 0,
                    nf: true,
                    hf: (a ^ r8 ^ res8) & 0x10 != 0,
                    cf: res > 0xff
                };
            }
        }
    };
}

macro_rules! and_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<and_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let res = regs![cpu.a] & quasi_r8s!(cpu/sys_state, $r);
                regs![res => cpu.a];

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: true,
                    cf: false
                };
            }
        }
    };
}

macro_rules! xor_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<xor_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let res = regs![cpu.a] ^ quasi_r8s!(cpu/sys_state, $r);
                regs![res => cpu.a];

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: false,
                    cf: false
                };
            }
        }
    };
}

macro_rules! or_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<or_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let res = regs![cpu.a] | quasi_r8s!(cpu/sys_state, $r);
                regs![res => cpu.a];

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: false,
                    cf: false
                };
            }
        }
    };
}

macro_rules! cp_a_r8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<cp_a_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let a = regs![cpu.a];
                let r8 = quasi_r8s!(cpu/sys_state, $r);
                let (res, cf) = a.overflowing_sub(r8);

                flags! { cpu;
                    zf: res == 0,
                    nf: true,
                    hf: (a ^ r8 ^ res) & 0x10 != 0,
                    cf: cf
                };
            }
        }
    };
}

macro_rules! rotate8_result {
    ($cpu:ident, $v:expr, l, to_carry) => (($v << 1) | ($v >> 7));
    ($cpu:ident, $v:expr, r, to_carry) => (($v >> 1) | ($v << 7));
    ($cpu:ident, $v:expr, l, with_carry)
        => (($v << 1) | if flags!($cpu.cf) { 0x01u8 } else { 0x00u8 });
    ($cpu:ident, $v:expr, r, with_carry)
        => (($v >> 1) | if flags!($cpu.cf) { 0x80u8 } else { 0x00u8 });
}

macro_rules! rotate8_zf {
    ($res:expr, short) => (false);
    ($res:expr, prefixed) => ($res == 0);
}

macro_rules! shift8_cf {
    ($src:expr, l) => ($src & 0x80u8 != 0);
    ($src:expr, r) => ($src & 0x01u8 != 0);
}

macro_rules! rotate8 {
    ($name: ident, $r:ident, $dir:ident, $carry:ident, $shorthand:ident) => {
        #[allow(unused_variables)]
        fn $name(cpu: &mut CPU, sys_state: &mut SystemState) {
            let src = quasi_r8s!(cpu/sys_state, $r);
            let res = rotate8_result!(cpu, src, $dir, $carry);
            quasi_r8d!(cpu/sys_state, $r, res);

            flags! { cpu;
                zf: rotate8_zf!(res, $shorthand),
                nf: false,
                hf: false,
                cf: shift8_cf!(src, $dir)
            };
        }
    };
}

macro_rules! prefixed_rot8 {
    ($dir:ident, to_carry, $r:ident) => {
        paste::item! {
            rotate8!([<r $dir c_ $r>], $r, $dir, to_carry, prefixed);
        }
    };

    ($dir:ident, with_carry, $r:ident) => {
        paste::item! {
            rotate8!([<r $dir _ $r>], $r, $dir, with_carry, prefixed);
        }
    };
}

macro_rules! shift8_result {
    ($v:expr, l, a) => ($v << 1);
    ($v:expr, r, a) => ((($v as i8) >> 1) as u8);
    ($v:expr, r, l) => ($v >> 1);
}

macro_rules! shift8 {
    ($dir:ident, $type:ident, $r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<s $dir $type _ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let src = quasi_r8s!(cpu/sys_state, $r);
                let res = shift8_result!(src, $dir, $type);
                quasi_r8d!(cpu/sys_state, $r, res);

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: false,
                    cf: shift8_cf!(src, $dir)
                };
            }
        }
    };
}

macro_rules! swap8 {
    ($r:ident) => {
        paste::item! {
            #[allow(unused_variables)]
            fn [<swap_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                let src = quasi_r8s!(cpu/sys_state, $r);
                let res = (src >> 4) | (src << 4);
                quasi_r8d!(cpu/sys_state, $r, res);

                flags! { cpu;
                    zf: res == 0,
                    nf: false,
                    hf: false,
                    cf: false
                };
            }
        }
    };
}

macro_rules! inc_r16 {
    ($r:ident) => {
        paste::item! {
            fn [<inc_ $r>](cpu: &mut CPU, _sys_state: &mut SystemState) {
                regs![regs![cpu.$r].wrapping_add(1u16) => cpu.$r];
            }
        }
    };
}

macro_rules! dec_r16 {
    ($r:ident) => {
        paste::item! {
            fn [<dec_ $r>](cpu: &mut CPU, _sys_state: &mut SystemState) {
                regs![regs![cpu.$r].wrapping_sub(1u16) => cpu.$r];
            }
        }
    };
}

macro_rules! add_hl_r16 {
    ($r:ident) => {
        paste::item! {
            fn [<add_hl_ $r>](cpu: &mut CPU, _sys_state: &mut SystemState) {
                let hl = regs![cpu.hl];
                let r16 = regs![cpu.$r];
                let (res, cf) = hl.overflowing_add(r16);

                regs![res => cpu.hl];

                flags! { cpu;
                    nf: false,
                    hf: (hl ^ r16 ^ res) & 0x1000 != 0,
                    cf: cf
                };
            }
        }
    }
}

macro_rules! cond_op {
    ($name:ident, $cc:ident, $op:ident, $pc_skip:expr) => {
        fn $name(cpu: &mut CPU, sys_state: &mut SystemState) {
            if flags![cpu.$cc] {
                $op(cpu, sys_state);
            } else {
                let ofs = $pc_skip;
                regs![regs![cpu.pc].wrapping_add(ofs) => cpu.pc];
            }
        }
    };

    ($name:ident, !$cc:ident, $op:ident, $pc_skip:expr) => {
        fn $name(cpu: &mut CPU, sys_state: &mut SystemState) {
            if !flags![cpu.$cc] {
                $op(cpu, sys_state);
            } else {
                let ofs = $pc_skip;
                regs![regs![cpu.pc].wrapping_add(ofs) => cpu.pc];
            }
        }
    };
}

macro_rules! pop_r16 {
    ($r:ident) => {
        paste::item! {
            fn [<pop_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                regs![pop(cpu, sys_state) => cpu.$r];
            }
        }
    };
}

macro_rules! push_r16 {
    ($r:ident) => {
        paste::item! {
            fn [<push_ $r>](cpu: &mut CPU, sys_state: &mut SystemState) {
                push(cpu, sys_state, regs![cpu.$r]);
            }
        }
    };
}

macro_rules! rstn {
    ($ofs:expr) => {
        paste::item! {
            fn [<rst_ $ofs>](cpu: &mut CPU, sys_state: &mut SystemState) {
                push(cpu, sys_state, regs![cpu.pc]);
                regs![$ofs => cpu.pc];
            }
        }
    };
}


fn not_implemented(cpu: &mut CPU, sys_state: &mut SystemState) {
    regs![regs![cpu.pc].wrapping_sub(1u16) => cpu.pc];
    let insn = mem![sys_state; u8:regs![cpu.pc]];
    cpu_panic(cpu, format!("INSN 0x{:02x} not implemented", insn).as_str());
}

fn nop(_cpu: &mut CPU, _sys_state: &mut SystemState) {
}

ld_r8_r8!(a, a);
ld_r8_r8!(a, b);
ld_r8_r8!(a, c);
ld_r8_r8!(a, d);
ld_r8_r8!(a, e);
ld_r8_r8!(a, h);
ld_r8_r8!(a, l);
ld_r8_r8!(a, _bc);
ld_r8_r8!(a, _de);
ld_r8_r8!(a, _hl);
ld_r8_r8!(a, n8);
ld_r8_r8!(a, _ffn8);
ld_r8_r8!(a, _ffc);
ld_r8_r8!(a, _n16);

ld_r8_r8!(b, a);
ld_r8_r8!(b, b);
ld_r8_r8!(b, c);
ld_r8_r8!(b, d);
ld_r8_r8!(b, e);
ld_r8_r8!(b, h);
ld_r8_r8!(b, l);
ld_r8_r8!(b, _hl);
ld_r8_r8!(b, n8);

ld_r8_r8!(c, a);
ld_r8_r8!(c, b);
ld_r8_r8!(c, c);
ld_r8_r8!(c, d);
ld_r8_r8!(c, e);
ld_r8_r8!(c, h);
ld_r8_r8!(c, l);
ld_r8_r8!(c, _hl);
ld_r8_r8!(c, n8);

ld_r8_r8!(d, a);
ld_r8_r8!(d, b);
ld_r8_r8!(d, c);
ld_r8_r8!(d, d);
ld_r8_r8!(d, e);
ld_r8_r8!(d, h);
ld_r8_r8!(d, l);
ld_r8_r8!(d, _hl);
ld_r8_r8!(d, n8);

ld_r8_r8!(e, a);
ld_r8_r8!(e, b);
ld_r8_r8!(e, c);
ld_r8_r8!(e, d);
ld_r8_r8!(e, e);
ld_r8_r8!(e, h);
ld_r8_r8!(e, l);
ld_r8_r8!(e, _hl);
ld_r8_r8!(e, n8);

ld_r8_r8!(h, a);
ld_r8_r8!(h, b);
ld_r8_r8!(h, c);
ld_r8_r8!(h, d);
ld_r8_r8!(h, e);
ld_r8_r8!(h, h);
ld_r8_r8!(h, l);
ld_r8_r8!(h, _hl);
ld_r8_r8!(h, n8);

ld_r8_r8!(l, a);
ld_r8_r8!(l, b);
ld_r8_r8!(l, c);
ld_r8_r8!(l, d);
ld_r8_r8!(l, e);
ld_r8_r8!(l, h);
ld_r8_r8!(l, l);
ld_r8_r8!(l, _hl);
ld_r8_r8!(l, n8);

ld_r8_r8!(_hl, a);
ld_r8_r8!(_hl, b);
ld_r8_r8!(_hl, c);
ld_r8_r8!(_hl, d);
ld_r8_r8!(_hl, e);
ld_r8_r8!(_hl, h);
ld_r8_r8!(_hl, l);
ld_r8_r8!(_hl, n8);

ld_r8_r8!(_bc, a);
ld_r8_r8!(_de, a);
ld_r8_r8!(_ffn8, a);
ld_r8_r8!(_ffc, a);
ld_r8_r8!(_n16, a);

ld_r16_n16!(bc);
ld_r16_n16!(de);
ld_r16_n16!(hl);
ld_r16_n16!(sp);

inc_r8!(a);
inc_r8!(b);
inc_r8!(c);
inc_r8!(d);
inc_r8!(e);
inc_r8!(h);
inc_r8!(l);
inc_r8!(_hl);

dec_r8!(a);
dec_r8!(b);
dec_r8!(c);
dec_r8!(d);
dec_r8!(e);
dec_r8!(h);
dec_r8!(l);
dec_r8!(_hl);

add_a_r8!(a);
add_a_r8!(b);
add_a_r8!(c);
add_a_r8!(d);
add_a_r8!(e);
add_a_r8!(h);
add_a_r8!(l);
add_a_r8!(_hl);
add_a_r8!(n8);

adc_a_r8!(a);
adc_a_r8!(b);
adc_a_r8!(c);
adc_a_r8!(d);
adc_a_r8!(e);
adc_a_r8!(h);
adc_a_r8!(l);
adc_a_r8!(_hl);
adc_a_r8!(n8);

sub_a_r8!(a);
sub_a_r8!(b);
sub_a_r8!(c);
sub_a_r8!(d);
sub_a_r8!(e);
sub_a_r8!(h);
sub_a_r8!(l);
sub_a_r8!(_hl);
sub_a_r8!(n8);

sbc_a_r8!(a);
sbc_a_r8!(b);
sbc_a_r8!(c);
sbc_a_r8!(d);
sbc_a_r8!(e);
sbc_a_r8!(h);
sbc_a_r8!(l);
sbc_a_r8!(_hl);
sbc_a_r8!(n8);

and_a_r8!(a);
and_a_r8!(b);
and_a_r8!(c);
and_a_r8!(d);
and_a_r8!(e);
and_a_r8!(h);
and_a_r8!(l);
and_a_r8!(_hl);
and_a_r8!(n8);

xor_a_r8!(a);
xor_a_r8!(b);
xor_a_r8!(c);
xor_a_r8!(d);
xor_a_r8!(e);
xor_a_r8!(h);
xor_a_r8!(l);
xor_a_r8!(_hl);
xor_a_r8!(n8);

or_a_r8!(a);
or_a_r8!(b);
or_a_r8!(c);
or_a_r8!(d);
or_a_r8!(e);
or_a_r8!(h);
or_a_r8!(l);
or_a_r8!(_hl);
or_a_r8!(n8);

cp_a_r8!(a);
cp_a_r8!(b);
cp_a_r8!(c);
cp_a_r8!(d);
cp_a_r8!(e);
cp_a_r8!(h);
cp_a_r8!(l);
cp_a_r8!(_hl);
cp_a_r8!(n8);

inc_r16!(bc);
inc_r16!(de);
inc_r16!(hl);
inc_r16!(sp);

dec_r16!(bc);
dec_r16!(de);
dec_r16!(hl);
dec_r16!(sp);

add_hl_r16!(bc);
add_hl_r16!(de);
add_hl_r16!(hl);
add_hl_r16!(sp);

fn add_sp_n8_helper(cpu: &mut CPU, sys_state: &mut SystemState) -> u16 {
    let sp = regs![cpu.sp];
    let n8 = (n8(cpu, sys_state) as i8) as u16;
    let res = sp.wrapping_add(n8);

    flags! { cpu;
        zf: false,
        nf: false,
        /* Uses 8-bit addition CF/HF for some reason */
        hf: (sp ^ n8 ^ res) & 0x10 != 0,
        cf: (sp ^ n8 ^ res) & 0x100 != 0
    };

    res
}

fn add_sp_n8(cpu: &mut CPU, sys_state: &mut SystemState) {
    regs![add_sp_n8_helper(cpu, sys_state) => cpu.sp];
}

fn ld_hl_spn8(cpu: &mut CPU, sys_state: &mut SystemState) {
    regs![add_sp_n8_helper(cpu, sys_state) => cpu.hl];
}

fn ld_sp_hl(cpu: &mut CPU, _sys_state: &mut SystemState) {
    regs![regs![cpu.hl] => cpu.sp];
}

rotate8!(rlca, a, l, to_carry, short);
rotate8!(rrca, a, r, to_carry, short);
rotate8!(rla, a, l, with_carry, short);
rotate8!(rra, a, r, with_carry, short);

prefixed_rot8!(l, to_carry, a);
prefixed_rot8!(l, to_carry, b);
prefixed_rot8!(l, to_carry, c);
prefixed_rot8!(l, to_carry, d);
prefixed_rot8!(l, to_carry, e);
prefixed_rot8!(l, to_carry, h);
prefixed_rot8!(l, to_carry, l);
prefixed_rot8!(l, to_carry, _hl);

prefixed_rot8!(r, to_carry, a);
prefixed_rot8!(r, to_carry, b);
prefixed_rot8!(r, to_carry, c);
prefixed_rot8!(r, to_carry, d);
prefixed_rot8!(r, to_carry, e);
prefixed_rot8!(r, to_carry, h);
prefixed_rot8!(r, to_carry, l);
prefixed_rot8!(r, to_carry, _hl);

prefixed_rot8!(l, with_carry, a);
prefixed_rot8!(l, with_carry, b);
prefixed_rot8!(l, with_carry, c);
prefixed_rot8!(l, with_carry, d);
prefixed_rot8!(l, with_carry, e);
prefixed_rot8!(l, with_carry, h);
prefixed_rot8!(l, with_carry, l);
prefixed_rot8!(l, with_carry, _hl);

prefixed_rot8!(r, with_carry, a);
prefixed_rot8!(r, with_carry, b);
prefixed_rot8!(r, with_carry, c);
prefixed_rot8!(r, with_carry, d);
prefixed_rot8!(r, with_carry, e);
prefixed_rot8!(r, with_carry, h);
prefixed_rot8!(r, with_carry, l);
prefixed_rot8!(r, with_carry, _hl);

shift8!(l, a, a);
shift8!(l, a, b);
shift8!(l, a, c);
shift8!(l, a, d);
shift8!(l, a, e);
shift8!(l, a, h);
shift8!(l, a, l);
shift8!(l, a, _hl);

shift8!(r, a, a);
shift8!(r, a, b);
shift8!(r, a, c);
shift8!(r, a, d);
shift8!(r, a, e);
shift8!(r, a, h);
shift8!(r, a, l);
shift8!(r, a, _hl);

shift8!(r, l, a);
shift8!(r, l, b);
shift8!(r, l, c);
shift8!(r, l, d);
shift8!(r, l, e);
shift8!(r, l, h);
shift8!(r, l, l);
shift8!(r, l, _hl);

swap8!(a);
swap8!(b);
swap8!(c);
swap8!(d);
swap8!(e);
swap8!(h);
swap8!(l);
swap8!(_hl);

fn ld__n16_sp(cpu: &mut CPU, sys_state: &mut SystemState) {
    let n = n16(cpu, sys_state);
    mem![sys_state; regs![cpu.sp] => u16:n];
}

fn ldi_a__hl(cpu: &mut CPU, sys_state: &mut SystemState) {
    let hl = regs![cpu.hl];
    regs![mem![sys_state; u8:hl] => cpu.a];
    regs![hl.wrapping_add(1u16) => cpu.hl];
}

fn ldi__hl_a(cpu: &mut CPU, sys_state: &mut SystemState) {
    let hl = regs![cpu.hl];
    mem![sys_state; regs![cpu.a] => u8:hl];
    regs![hl.wrapping_add(1u16) => cpu.hl];
}

fn ldd_a__hl(cpu: &mut CPU, sys_state: &mut SystemState) {
    let hl = regs![cpu.hl];
    regs![mem![sys_state; u8:hl] => cpu.a];
    regs![hl.wrapping_sub(1u16) => cpu.hl];
}

fn ldd__hl_a(cpu: &mut CPU, sys_state: &mut SystemState) {
    let hl = regs![cpu.hl];
    mem![sys_state; regs![cpu.a] => u8:hl];
    regs![hl.wrapping_sub(1u16) => cpu.hl];
}

fn cpl(cpu: &mut CPU, _sys_state: &mut SystemState) {
    regs![!regs![cpu.a] => cpu.a];

    flags! { cpu;
        nf: true,
        hf: true
    };
}

fn daa(cpu: &mut CPU, _sys_state: &mut SystemState) {
    let mut a = regs![cpu.a] as u32;

    if !flags![cpu.nf] {
        if (a & 0xf) > 0x9 || flags![cpu.hf] {
            a += 0x6;
        }
        if a > 0x99 || flags![cpu.cf] {
            a += 0x60;
        }

        flags! { cpu;
            cf: (a & 0x100) != 0,
            hf: false,
            zf: a == 0
        };
    } else {
        if flags![cpu.hf] {
            a = a.wrapping_sub(0x6);
        }
        if flags![cpu.cf] {
            a = a.wrapping_sub(0x60);
        }

        flags! { cpu;
            hf: false,
            zf: a == 0
        };
    }

    regs![a as u8 => cpu.a];
}

fn ccf(cpu: &mut CPU, _sys_state: &mut SystemState) {
    flags! { cpu;
        nf: false,
        hf: false,
        cf: !flags![cpu.cf]
    };
}

fn scf(cpu: &mut CPU, _sys_state: &mut SystemState) {
    flags! { cpu;
        nf: false,
        hf: false,
        cf: true
    };
}

fn jr_n8(cpu: &mut CPU, sys_state: &mut SystemState) {
    let ofs = n8(cpu, sys_state) as i8;
    regs![regs![cpu.pc].wrapping_add(ofs as u16) => cpu.pc];
}

cond_op!(jrnz_n8, !zf, jr_n8, 1);
cond_op!(jrz_n8,   zf, jr_n8, 1);
cond_op!(jrnc_n8, !cf, jr_n8, 1);
cond_op!(jrc_n8,   cf, jr_n8, 1);

fn jp_n16(cpu: &mut CPU, sys_state: &mut SystemState) {
    regs![n16(cpu, sys_state) => cpu.pc];
}

cond_op!(jpnz_n16, !zf, jp_n16, 2);
cond_op!(jpz_n16,   zf, jp_n16, 2);
cond_op!(jpnc_n16, !cf, jp_n16, 2);
cond_op!(jpc_n16,   cf, jp_n16, 2);

fn jp__hl(cpu: &mut CPU, _sys_state: &mut SystemState) {
    /* Caution: Actually, this is more of a "JP HL" than "JP (HL)" */
    regs![regs![cpu.hl] => cpu.pc];
}

fn pop(cpu: &mut CPU, sys_state: &mut SystemState) -> u16 {
    let sp = regs![cpu.sp];
    let val = mem![sys_state; u16:sp];
    regs![sp.wrapping_add(2) => cpu.sp];
    val
}

pub fn push(cpu: &mut CPU, sys_state: &mut SystemState, val: u16) {
    let sp = regs![cpu.sp].wrapping_sub(2);
    regs![sp => cpu.sp];
    mem![sys_state; val => u16:sp];
}

fn call_n16(cpu: &mut CPU, sys_state: &mut SystemState) {
    let dst = n16(cpu, sys_state);
    push(cpu, sys_state, regs![cpu.pc]);
    regs![dst => cpu.pc];
}

fn ret(cpu: &mut CPU, sys_state: &mut SystemState) {
    regs![pop(cpu, sys_state) => cpu.pc];
}

cond_op!(callnz_n16, !zf, call_n16, 2);
cond_op!(callz_n16,   zf, call_n16, 2);
cond_op!(callnc_n16, !cf, call_n16, 2);
cond_op!(callc_n16,   cf, call_n16, 2);

rstn!(0x00);
rstn!(0x08);
rstn!(0x10);
rstn!(0x18);
rstn!(0x20);
rstn!(0x28);
rstn!(0x30);
rstn!(0x38);

cond_op!(retnz, !zf, ret, 0);
cond_op!(retz,   zf, ret, 0);
cond_op!(retnc, !cf, ret, 0);
cond_op!(retc,   cf, ret, 0);

fn reti(cpu: &mut CPU, sys_state: &mut SystemState) {
    ret(cpu, sys_state);

    sys_state.ints_enabled = true;
}

pop_r16!(af);
pop_r16!(bc);
pop_r16!(de);
pop_r16!(hl);

push_r16!(af);
push_r16!(bc);
push_r16!(de);
push_r16!(hl);

fn ei(cpu: &mut CPU, _sys_state: &mut SystemState) {
    cpu.inject_int_insn(1, IIOperation::EnableInterrupts);
}

fn di(cpu: &mut CPU, _sys_state: &mut SystemState) {
    cpu.inject_int_insn(1, IIOperation::DisableInterrupts);
}

fn halt(cpu: &mut CPU, _sys_state: &mut SystemState) {
    cpu.halted = true;
}


const INSN_HANDLERS: [fn(&mut CPU, &mut SystemState); 256] = [
    nop,                /* 0x00 */
    ld_bc_n16,
    ld__bc_a,
    inc_bc,
    inc_b,
    dec_b,
    ld_b_n8,
    rlca,
    ld__n16_sp,          /* 0x08 */
    add_hl_bc,
    ld_a__bc,
    dec_bc,
    inc_c,
    dec_c,
    ld_c_n8,
    rrca,
    prefix0x10,         /* 0x10 */
    ld_de_n16,
    ld__de_a,
    inc_de,
    inc_d,
    dec_d,
    ld_d_n8,
    rla,
    jr_n8,              /* 0x18 */
    add_hl_de,
    ld_a__de,
    dec_de,
    inc_e,
    dec_e,
    ld_e_n8,
    rra,
    jrnz_n8,            /* 0x20 */
    ld_hl_n16,
    ldi__hl_a,
    inc_hl,
    inc_h,
    dec_h,
    ld_h_n8,
    daa,
    jrz_n8,             /* 0x28 */
    add_hl_hl,
    ldi_a__hl,
    dec_hl,
    inc_l,
    dec_l,
    ld_l_n8,
    cpl,
    jrnc_n8,            /* 0x30 */
    ld_sp_n16,
    ldd__hl_a,
    inc_sp,
    inc__hl,
    dec__hl,
    ld__hl_n8,
    scf,
    jrc_n8,             /* 0x38 */
    add_hl_sp,
    ldd_a__hl,
    dec_sp,
    inc_a,
    dec_a,
    ld_a_n8,
    ccf,
    ld_b_b,             /* 0x40 */
    ld_b_c,
    ld_b_d,
    ld_b_e,
    ld_b_h,
    ld_b_l,
    ld_b__hl,
    ld_b_a,
    ld_c_b,             /* 0x48 */
    ld_c_c,
    ld_c_d,
    ld_c_e,
    ld_c_h,
    ld_c_l,
    ld_c__hl,
    ld_c_a,
    ld_d_b,             /* 0x50 */
    ld_d_c,
    ld_d_d,
    ld_d_e,
    ld_d_h,
    ld_d_l,
    ld_d__hl,
    ld_d_a,
    ld_e_b,             /* 0x58 */
    ld_e_c,
    ld_e_d,
    ld_e_e,
    ld_e_h,
    ld_e_l,
    ld_e__hl,
    ld_e_a,
    ld_h_b,             /* 0x60 */
    ld_h_c,
    ld_h_d,
    ld_h_e,
    ld_h_h,
    ld_h_l,
    ld_h__hl,
    ld_h_a,
    ld_l_b,             /* 0x68 */
    ld_l_c,
    ld_l_d,
    ld_l_e,
    ld_l_h,
    ld_l_l,
    ld_l__hl,
    ld_l_a,
    ld__hl_b,           /* 0x70 */
    ld__hl_c,
    ld__hl_d,
    ld__hl_e,
    ld__hl_h,
    ld__hl_l,
    halt,
    ld__hl_a,
    ld_a_b,             /* 0x78 */
    ld_a_c,
    ld_a_d,
    ld_a_e,
    ld_a_h,
    ld_a_l,
    ld_a__hl,
    ld_a_a,
    add_a_b,            /* 0x80 */
    add_a_c,
    add_a_d,
    add_a_e,
    add_a_h,
    add_a_l,
    add_a__hl,
    add_a_a,
    adc_a_b,            /* 0x88 */
    adc_a_c,
    adc_a_d,
    adc_a_e,
    adc_a_h,
    adc_a_l,
    adc_a__hl,
    adc_a_a,
    sub_a_b,            /* 0x90 */
    sub_a_c,
    sub_a_d,
    sub_a_e,
    sub_a_h,
    sub_a_l,
    sub_a__hl,
    sub_a_a,
    sbc_a_b,            /* 0x98 */
    sbc_a_c,
    sbc_a_d,
    sbc_a_e,
    sbc_a_h,
    sbc_a_l,
    sbc_a__hl,
    sbc_a_a,
    and_a_b,            /* 0xa0 */
    and_a_c,
    and_a_d,
    and_a_e,
    and_a_h,
    and_a_l,
    and_a__hl,
    and_a_a,
    xor_a_b,            /* 0xa8 */
    xor_a_c,
    xor_a_d,
    xor_a_e,
    xor_a_h,
    xor_a_l,
    xor_a__hl,
    xor_a_a,
    or_a_b,             /* 0xb0 */
    or_a_c,
    or_a_d,
    or_a_e,
    or_a_h,
    or_a_l,
    or_a__hl,
    or_a_a,
    cp_a_b,             /* 0xb8 */
    cp_a_c,
    cp_a_d,
    cp_a_e,
    cp_a_h,
    cp_a_l,
    cp_a__hl,
    cp_a_a,
    retnz,              /* 0xc0 */
    pop_bc,
    jpnz_n16,
    jp_n16,
    callnz_n16,
    push_bc,
    add_a_n8,
    rst_0x00,
    retz,               /* 0xc8 */
    ret,
    jpz_n16,
    not_implemented, /* actually prefix0xcb */
    callz_n16,
    call_n16,
    adc_a_n8,
    rst_0x08,
    retnc,              /* 0xd0 */
    pop_de,
    jpnc_n16,
    not_implemented,
    callnc_n16,
    push_de,
    sub_a_n8,
    rst_0x10,
    retc,               /* 0xd8 */
    reti,
    jpc_n16,
    not_implemented,
    callc_n16,
    not_implemented,
    sbc_a_n8,
    rst_0x18,
    ld__ffn8_a,         /* 0xe0 */
    pop_hl,
    ld__ffc_a,
    not_implemented,
    not_implemented,
    push_hl,
    and_a_n8,
    rst_0x20,
    add_sp_n8,          /* 0xe8 */
    jp__hl,
    ld__n16_a,
    not_implemented,
    not_implemented,
    not_implemented,
    xor_a_n8,
    rst_0x28,
    ld_a__ffn8,         /* 0xf0 */
    pop_af,
    ld_a__ffc,
    di,
    not_implemented,
    push_af,
    or_a_n8,
    rst_0x30,
    ld_hl_spn8,         /* 0xf8 */
    ld_sp_hl,
    ld_a__n16,
    ei,
    not_implemented,
    not_implemented,
    cp_a_n8,
    rst_0x38
];

const INSN_CB_HANDLERS: [fn(&mut CPU, &mut SystemState); 64] = [
    rlc_b,              /* 0x00 */
    rlc_c,
    rlc_d,
    rlc_e,
    rlc_h,
    rlc_l,
    rlc__hl,
    rlc_a,
    rrc_b,              /* 0x08 */
    rrc_c,
    rrc_d,
    rrc_e,
    rrc_h,
    rrc_l,
    rrc__hl,
    rrc_a,
    rl_b,               /* 0x10 */
    rl_c,
    rl_d,
    rl_e,
    rl_h,
    rl_l,
    rl__hl,
    rl_a,
    rr_b,               /* 0x18 */
    rr_c,
    rr_d,
    rr_e,
    rr_h,
    rr_l,
    rr__hl,
    rr_a,
    sla_b,              /* 0x20 */
    sla_c,
    sla_d,
    sla_e,
    sla_h,
    sla_l,
    sla__hl,
    sla_a,
    sra_b,              /* 0x28 */
    sra_c,
    sra_d,
    sra_e,
    sra_h,
    sra_l,
    sra__hl,
    sra_a,
    swap_b,             /* 0x30 */
    swap_c,
    swap_d,
    swap_e,
    swap_h,
    swap_l,
    swap__hl,
    swap_a,
    srl_b,              /* 0x38 */
    srl_c,
    srl_d,
    srl_e,
    srl_h,
    srl_l,
    srl__hl,
    srl_a
];

const INSN_CYCLES: [u8; 256] = [
    1, 3, 2, 2, 1, 1, 2, 1, 5, 2, 2, 2, 1, 1, 2, 1,
    0, 3, 2, 2, 1, 1, 2, 1, 2, 2, 2, 2, 1, 1, 2, 1,
    2, 3, 2, 2, 1, 1, 2, 1, 2, 2, 2, 2, 1, 1, 2, 1,
    2, 3, 2, 2, 3, 3, 3, 1, 2, 2, 2, 2, 1, 1, 2, 1,

    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 1, 1, 1, 1, 2, 1,

    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,
    1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1,

    2, 3, 3, 3, 3, 4, 2, 8, 2, 2, 3, 1, 3, 3, 2, 8,
    2, 3, 3, 0, 3, 4, 2, 8, 2, 2, 3, 0, 3, 0, 2, 8,
    3, 3, 2, 0, 0, 4, 2, 8, 4, 1, 3, 0, 0, 0, 2, 8,
    3, 3, 2, 1, 0, 4, 2, 8, 3, 2, 3, 1, 0, 0, 2, 8,
];

const INSN_CB_CYCLES: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1,
    1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1,
    1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1,
    1, 1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 3, 1,

    1, 1, 1, 1, 1, 1, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,

    1, 1, 1, 1, 1, 1, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,

    1, 1, 1, 1, 1, 1, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
