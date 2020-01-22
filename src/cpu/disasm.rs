use crate::{mem, regs, regs8, regs16, regs16_split};
#[cfg(target_os = "linux")]
use crate::address_space::AS_BASE;
use crate::cpu::CPU;
use crate::io::io_read;
use crate::system_state::SystemState;

fn peek8(sys_state: &mut SystemState, base: u16, ofs: u16) -> u8 {
    mem![sys_state; base.wrapping_add(ofs)]
}

fn peek16(sys_state: &mut SystemState, base: u16, ofs: u16) -> u16 {
    let lo = { mem![sys_state; base.wrapping_add(ofs)] };
    let hi = { mem![sys_state; base.wrapping_add(ofs).wrapping_add(1)] };

    (lo as u16) | ((hi as u16) << 8)
}

fn r8(code: u8) -> &'static str {
    match code {
        0x0 => "b",
        0x1 => "c",
        0x2 => "d",
        0x3 => "e",
        0x4 => "h",
        0x5 => "l",
        0x6 => "(hl)",
        0x7 => "a",

        _ => unreachable!(),
    }
}

fn r16_sp(code: u8) -> &'static str {
    match code {
        0x0 => "bc",
        0x1 => "de",
        0x2 => "hl",
        0x3 => "sp",

        _ => unreachable!(),
    }
}

fn r16_af(code: u8) -> &'static str {
    match code {
        0x0 => "bc",
        0x1 => "de",
        0x2 => "hl",
        0x3 => "af",

        _ => unreachable!(),
    }
}

fn ioreg_name(reg: u8) -> String {
    match reg {
        0x00 => String::from("P1"),
        0x01 => String::from("SB"),
        0x02 => String::from("SC"),
        0x04 => String::from("DIV"),
        0x05 => String::from("TIMA"),
        0x06 => String::from("TMA"),
        0x07 => String::from("TAC"),
        0x0f => String::from("IF"),
        0x10 => String::from("NR10"),
        0x11 => String::from("NR11"),
        0x12 => String::from("NR12"),
        0x13 => String::from("NR13"),
        0x14 => String::from("NR14"),
        0x15 => String::from("NR20"),
        0x16 => String::from("NR21"),
        0x17 => String::from("NR22"),
        0x18 => String::from("NR23"),
        0x19 => String::from("NR24"),
        0x1a => String::from("NR30"),
        0x1b => String::from("NR31"),
        0x1c => String::from("NR32"),
        0x1d => String::from("NR33"),
        0x1e => String::from("NR34"),
        0x1f => String::from("NR40"),
        0x20 => String::from("NR41"),
        0x21 => String::from("NR42"),
        0x22 => String::from("NR43"),
        0x23 => String::from("NR44"),
        0x24 => String::from("NR50"),
        0x25 => String::from("NR51"),
        0x26 => String::from("NR52"),
        0x30 => String::from("WAVE00"),
        0x31 => String::from("WAVE02"),
        0x32 => String::from("WAVE04"),
        0x33 => String::from("WAVE06"),
        0x34 => String::from("WAVE08"),
        0x35 => String::from("WAVE0a"),
        0x36 => String::from("WAVE0c"),
        0x37 => String::from("WAVE0e"),
        0x38 => String::from("WAVE10"),
        0x39 => String::from("WAVE12"),
        0x3a => String::from("WAVE14"),
        0x3b => String::from("WAVE16"),
        0x3c => String::from("WAVE18"),
        0x3d => String::from("WAVE1a"),
        0x3e => String::from("WAVE1c"),
        0x3f => String::from("WAVE1e"),
        0x40 => String::from("LCDC"),
        0x41 => String::from("STAT"),
        0x42 => String::from("SCY"),
        0x43 => String::from("SCX"),
        0x44 => String::from("LY"),
        0x45 => String::from("LYC"),
        0x46 => String::from("DMA"),
        0x47 => String::from("BGP"),
        0x48 => String::from("OBP0"),
        0x49 => String::from("OBP1"),
        0x4a => String::from("WY"),
        0x4b => String::from("WX"),
        0x4d => String::from("KEY1"),
        0x4f => String::from("VBK"),
        0x51 => String::from("HDMA1"),
        0x52 => String::from("HDMA2"),
        0x53 => String::from("HDMA3"),
        0x54 => String::from("HDMA4"),
        0x55 => String::from("HDMA5"),
        0x56 => String::from("RP"),
        0x68 => String::from("BCPS"),
        0x69 => String::from("BCPD"),
        0x6a => String::from("OCPS"),
        0x6b => String::from("OCPD"),
        0x70 => String::from("SVBK"),
        0xff => String::from("IE"),

        0x80..=0xfe => format!("hmem 0x{:x}", reg),

        _ => format!("unknown register 0x{:02x}", reg),
    }
}

fn disasm_prefix_0x10(sys_state: &mut SystemState, cpu: &CPU) -> String {
    let op = mem![sys_state; cpu.pc.wrapping_add(1)];

    String::from(
        match op {
            0x00 => "stop",
            _ => "db   0x10",
        }
    )
}

fn disasm_prefix_0xcb(sys_state: &mut SystemState, cpu: &CPU) -> String {
    let op = mem![sys_state; cpu.pc.wrapping_add(1)];
    let r8_op = op & 0x07;
    let bit = (op & 0x38) >> 3;

    match op & 0xc0 {
        0x00 => {
            let mnemonic = match op & 0x38 {
                0x00 => "rlc",
                0x08 => "rrc",
                0x10 => "rl",
                0x18 => "rr",
                0x20 => "sla",
                0x28 => "sra",
                0x30 => "swap",
                0x38 => "srl",

                _ => unreachable!(),
            };

            format!("{:-6} {}", mnemonic, r8(r8_op))
        },

        0x40 => format!("bit    {}, {} # 0x{:02x}", r8(r8_op), bit, 1 << bit),
        0x80 => format!("res    {}, {} # 0x{:02x}", r8(r8_op), bit, 1 << bit),
        0xc0 => format!("set    {}, {} # 0x{:02x}", r8(r8_op), bit, 1 << bit),

        _ => unreachable!(),
    }
}

fn disasm_block_misc_lo(sys_state: &mut SystemState, cpu: &CPU, op: u8)
    -> String
{
    let n8 = peek8(sys_state, cpu.pc, 1);
    let n16 = peek16(sys_state, cpu.pc, 1);
    let jr_to = cpu.pc.wrapping_add(2).wrapping_add(n8 as i8 as u16);

    let r8_op = (op >> 3) & 0x07;
    let r16_op = (op >> 4) & 0x03;

    match op & 0x0f {
        0x00 | 0x08 => match op {
            0x00 => String::from("nop"),
            0x08 => format!("ld     (0x{:04x}), sp", n16),
            0x10 => disasm_prefix_0x10(sys_state, cpu),
            0x18 => format!("jr     0x{:04x}", jr_to),
            0x20 => format!("jrnz   0x{:04x}", jr_to),
            0x28 => format!("jrz    0x{:04x}", jr_to),
            0x30 => format!("jrnc   0x{:04x}", jr_to),
            0x38 => format!("jrc    0x{:04x}", jr_to),

            _ => unreachable!(),
        },

        0x01 => format!("ld     {}, 0x{:x} # {}",
                        r16_sp(r16_op), n16, n16 as i16),

        0x09 => format!("add    hl, {}", r16_sp(r16_op)),

        0x02 | 0x0a => match op {
            0x02 | 0x12 => format!("ld     ({}), a", r16_sp(r16_op)),
            0x0a | 0x1a => format!("ld     a, ({})", r16_sp(r16_op)),
            0x22 => String::from("ldi    (hl), a"),
            0x2a => String::from("ldi    a, (hl)"),
            0x32 => String::from("ldd    (hl), a"),
            0x3a => String::from("ldd    a, (hl)"),

            _ => unreachable!(),
        },

        0x03 => format!("inc    {}", r16_sp(r16_op)),

        0x0b => format!("dec    {}", r16_sp(r16_op)),

        0x04 | 0x0c => format!("inc    {}", r8(r8_op)),

        0x05 | 0x0d => format!("dec    {}", r8(r8_op)),

        0x06 | 0x0e => format!("ld     {}, 0x{:x} # {}",
                               r8(r8_op), n8, n8 as i8),

        0x07 | 0x0f => String::from(
            match op {
                0x07 => "rlca",
                0x0f => "rrca",
                0x17 => "rla",
                0x1f => "rra",
                0x27 => "daa",
                0x2f => "cpl",
                0x37 => "scf",
                0x3f => "ccf",

                _ => unreachable!(),
            }
        ),

        _ => unreachable!(),
    }
}

fn disasm_block_mov(_sys_state: &mut SystemState, _cpu: &CPU, op: u8) -> String {
    if op == 0x76 {
        String::from("halt")
    } else {
        let r8_op_src = (op >> 0) & 0x07;
        let r8_op_dst = (op >> 3) & 0x07;

        format!("ld     {}, {}", r8(r8_op_dst), r8(r8_op_src))
    }
}

fn disasm_block_alu(_sys_state: &mut SystemState, _cpu: &CPU, op: u8) -> String {
    let r8_op = op & 0x07;

    match op & 0x38 {
        0x00 => format!("add    a, {}", r8(r8_op)),
        0x08 => format!("adc    a, {}", r8(r8_op)),
        0x10 => format!("sub    a, {}", r8(r8_op)),
        0x18 => format!("sbc    a, {}", r8(r8_op)),
        0x20 => format!("and    a, {}", r8(r8_op)),
        0x28 => format!("xor    a, {}", r8(r8_op)),
        0x30 => format!("or     a, {}", r8(r8_op)),
        0x38 => format!("cp     a, {}", r8(r8_op)),

        _ => unreachable!(),
    }
}

fn disasm_block_misc_hi(sys_state: &mut SystemState, cpu: &CPU, op: u8)
    -> String
{
    let n8 = peek8(sys_state, cpu.pc, 1);
    let n16 = peek16(sys_state, cpu.pc, 1);
    let r16_op = (op >> 4) & 0x03;

    match op & 0x0f {
        0x00 | 0x08 => match op {
            0xc0 => String::from("retnz"),
            0xc8 => String::from("retz"),
            0xd0 => String::from("retnc"),
            0xd8 => String::from("retc"),
            0xe0 => format!("ld     (0xff{:02x}), a # {}", n8, ioreg_name(n8)),
            0xe8 => format!("add    sp, 0x{:x} # {}",
                            n8 as i8 as u16, n8 as i8),
            0xf0 => format!("ld     a, (0xff{:02x}) # {}", n8, ioreg_name(n8)),
            0xf8 => format!("ld     hl, sp + 0x{:x} # {}",
                            n8 as i8 as u16, n8 as i8),

            _ => unreachable!(),
        },

        0x01 => format!("pop    {}", r16_af(r16_op)),

        0x09 => match op {
            0xc9 => String::from("ret"),
            0xd9 => String::from("reti"),
            0xe9 => format!("jp    hl # 0x{:04x}", regs![cpu.hl]),
            0xf9 => String::from("ld    sp, hl"),

            _ => unreachable!(),
        },

        0x02 | 0x0a => match op {
            0xc2 => format!("jpnz   0x{:04x}", n16),
            0xca => format!("jpz    0x{:04x}", n16),
            0xd2 => format!("jpnc   0x{:04x}", n16),
            0xda => format!("jpc    0x{:04x}", n16),
            0xe2 => format!("ld     (0xff00 + c), a # {}", ioreg_name(regs![cpu.c])),
            0xea => format!("ld     (0x{:04x}), a", n16),
            0xf2 => format!("ld     a, (0xff00 + c) # {}", ioreg_name(regs![cpu.c])),
            0xfa => format!("ld     a, (0x{:04x})", n16),

            _ => unreachable!(),
        },

        0x03 => match op {
            0xc3 => format!("jp     0x{:04x}", n16),
            0xf3 => String::from("di"),

            0xd3 | 0xe3
                => format!("db    0x{:02x}", op),

            _ => unreachable!(),
        },

        0x0b => match op {
            0xcb => disasm_prefix_0xcb(sys_state, cpu),
            0xfb => String::from("ei"),

            0xdb | 0xeb
                => format!("db    0x{:02x}", op),

            _ => unreachable!(),
        },

        0x04 | 0x0c => match op {
            0xc4 => format!("callnz 0x{:04x}", n16),
            0xcc => format!("callz  0x{:04x}", n16),
            0xd4 => format!("callnc 0x{:04x}", n16),
            0xdc => format!("callc  0x{:04x}", n16),

            0xe4 | 0xec | 0xf4 | 0xfc
                => format!("db    0x{:02x}", op),

            _ => unreachable!(),
        },

        0x05 => format!("push   {}", r16_af(r16_op)),

        0x0d => match op {
            0xcd => format!("call   0x{:04x}", n16),

            0xdd | 0xed | 0xfd
                => format!("db    0x{:02x}", op),

            _ => unreachable!(),
        },

        0x06 | 0x0e => match op {
            0xc6 => format!("add    a, 0x{:02x} # {}", n8, n8 as i8),
            0xce => format!("adc    a, 0x{:02x} # {}", n8, n8 as i8),
            0xd6 => format!("sub    a, 0x{:02x} # {}", n8, n8 as i8),
            0xde => format!("sbc    a, 0x{:02x} # {}", n8, n8 as i8),
            0xe6 => format!("and    a, 0x{:02x} # {}", n8, n8 as i8),
            0xee => format!("xor    a, 0x{:02x} # {}", n8, n8 as i8),
            0xf6 => format!("or     a, 0x{:02x} # {}", n8, n8 as i8),
            0xfe => format!("cp     a, 0x{:02x} # {}", n8, n8 as i8),

            _ => unreachable!(),
        },

        0x07 | 0x0f => format!("rst    0x{:02x}", op & 0x38),

        _ => unreachable!(),
    }
}

pub fn disassemble(sys_state: &mut SystemState, cpu: &CPU) -> String {
    let op = mem![sys_state; cpu.pc];

    match op & 0xc0 {
        0x00 => disasm_block_misc_lo(sys_state, cpu, op),
        0x40 => disasm_block_mov(sys_state, cpu, op),
        0x80 => disasm_block_alu(sys_state, cpu, op),
        0xc0 => disasm_block_misc_hi(sys_state, cpu, op),

        _ => unreachable!(),
    }
}
