use savestate::SaveState;

use crate::system_state::SystemState;


struct PaletteData {
    data: [u8; 0x1000]
}

#[derive(SaveState)]
pub struct SGBState {
    packet_index: usize,
    packet_bit_index: usize,

    raw_packets: [[u8; 16]; 7],

    pal: PaletteData,
}

impl SGBState {
    pub fn new() -> Self {
        Self {
            packet_index: 0,
            packet_bit_index: 0,

            raw_packets: [[0u8; 16]; 7],

            pal: PaletteData {
                data: [0u8; 0x1000],
            },
        }
    }
}


impl SaveState for PaletteData {
    fn export<T: std::io::Write>(&self, stream: &mut T, _version: u64) {
        stream.write_all(&self.data).unwrap();
    }

    fn import<T: std::io::Read>(&mut self, stream: &mut T, _version: u64) {
        stream.read_exact(&mut self.data).unwrap();
    }
}


fn sgb_attr_blk(sys_state: &mut SystemState) {
    let s = &mut sys_state.sgb_state;

    sys_state.display.sgb_attr_blk(0b001, 0, 0, 0, 19, 17);

    let mut i = 2;
    for _ in 0..s.raw_packets[0][1] {
        sys_state.display.sgb_attr_blk(
            s.raw_packets[(i + 0) / 16][(i + 0) % 16],
            s.raw_packets[(i + 1) / 16][(i + 1) % 16],
            s.raw_packets[(i + 2) / 16][(i + 2) % 16] as usize,
            s.raw_packets[(i + 3) / 16][(i + 3) % 16] as usize,
            s.raw_packets[(i + 4) / 16][(i + 4) % 16] as usize,
            s.raw_packets[(i + 5) / 16][(i + 5) % 16] as usize);

        i += 6;
    }
}

fn sgb_pal_set(sys_state: &mut SystemState) {
    let s = &mut sys_state.sgb_state;
    let mut col0 = 0x7fff;

    if s.raw_packets[0][9] & 0xc0 != 0 {
        println!("Warning: SGB PAL_SET attribute file unhandled");
    }

    for pal_bi in 0..4 {
        let idx = s.raw_packets[0][pal_bi * 2 + 1] as usize |
                ((s.raw_packets[0][pal_bi * 2 + 2] as usize) << 8);

        for shade in 0..4 {
            let fpi = (idx * 4 + shade) * 2;
            let rgb15 = s.pal.data[fpi + 0] as u16 |
                      ((s.pal.data[fpi + 1] as u16) << 8);

            if shade == 0 && pal_bi == 3 {
                col0 = rgb15;
            }

            sys_state.display.set_bg_pal(pal_bi * 4 + shade, rgb15);
            sys_state.display.set_obj_pal(pal_bi * 4 + shade, rgb15);
        }
    }

    for pal_bi in 0..4 {
        sys_state.display.set_bg_pal(pal_bi * 4, col0);
        sys_state.display.set_obj_pal(pal_bi * 4, col0);
    }
}

fn sgb_pal_trn(sys_state: &mut SystemState) {
    sys_state.display.fill_for_sgb_buf = true;
}

fn sgb_mlt_req(sys_state: &mut SystemState) {
    match sys_state.sgb_state.raw_packets[0][1] {
        0 => sys_state.keypad.set_controller_count(1),
        1 => sys_state.keypad.set_controller_count(2),
        3 => sys_state.keypad.set_controller_count(4),

        _ => (),
    };
}

pub fn sgb_cmd(sys_state: &mut SystemState) {
    match sys_state.sgb_state.raw_packets[0][0] >> 3 {
        0x00 => println!("SGB PAL01 unhandled"),
        0x01 => println!("SGB PAL23 unhandled"),
        0x02 => println!("SGB PAL03 unhandled"),
        0x03 => println!("SGB PAL12 unhandled"),
        0x04 => sgb_attr_blk(sys_state),
        0x05 => println!("SGB ATTR_LIN unhandled"),
        0x06 => println!("SGB ATTR_DIV unhandled"),
        0x07 => println!("SGB ATTR_CHR unhandled"),
        0x08 => println!("SGB SOUND unhandled"),
        0x09 => println!("SGB SOU_TRN unhandled"),
        0x0a => sgb_pal_set(sys_state),
        0x0b => sgb_pal_trn(sys_state),
        0x0c => println!("SGB ATRC_EN unhandled"),
        0x0e => println!("SGB ICON_EN unhandled"),
        0x0f => println!("SGB DATA_SND unhandled"),
        0x10 => println!("SGB DATA_TRN unhandled"),
        0x11 => sgb_mlt_req(sys_state),
        0x12 => println!("SGB JUMP unhandled"),
        0x13 => println!("SGB CHR_TRN unhandled"),
        0x14 => println!("SGB PCT_TRN unhandled"),
        0x15 => println!("SGB ATTR_TRN unhandled"),
        0x16 => println!("SGB ATTR_SET unhandled"),
        0x17 => println!("SGB MASK_EN unhandled"),
        0x19 => println!("SGB PAL_PRI unhandled"),

        x => println!("Unknown SGB command {:02x}", x),
    }
}

pub fn sgb_pulse(sys_state: &mut SystemState, np14: bool, np15: bool) {
    let s = &mut sys_state.sgb_state;

    if np14 && np15 {
        s.packet_bit_index = 0;
        for x in &mut s.raw_packets[s.packet_index] {
            *x = 0;
        }
    } else if s.packet_bit_index < 16 * 8 {
        let pi = s.packet_index;
        if np15 {
            let i = s.packet_bit_index;
            s.raw_packets[pi][i / 8] |= 1 << (i % 8);
        }
        s.packet_bit_index += 1;

        if s.packet_bit_index == 16 * 8 {
            s.packet_index += 1;
            if s.packet_index >= (s.raw_packets[0][0] & 0x7) as usize {
                s.packet_index = 0;

                sgb_cmd(sys_state);
            }
        }
    }
}

/* FIXME: Allow transfers to things but the palette */
pub fn sgb_buf_done(sys_state: &mut SystemState) {
    let mut i = 0;
    let ibuf = &sys_state.display.for_sgb_buf;
    let obuf = &mut sys_state.sgb_state.pal.data;

    for by in (0..144).step_by(8) {
        for x in (0..160).step_by(8) {
            for ry in 0..8 {
                let y = by + ry;

                let p = (ibuf[y * 160 + x + 0],
                         ibuf[y * 160 + x + 1],
                         ibuf[y * 160 + x + 2],
                         ibuf[y * 160 + x + 3],
                         ibuf[y * 160 + x + 4],
                         ibuf[y * 160 + x + 5],
                         ibuf[y * 160 + x + 6],
                         ibuf[y * 160 + x + 7]);

                obuf[i] = ((p.0 & 1) << 7) |
                          ((p.1 & 1) << 6) |
                          ((p.2 & 1) << 5) |
                          ((p.3 & 1) << 4) |
                          ((p.4 & 1) << 3) |
                          ((p.5 & 1) << 2) |
                          ((p.6 & 1) << 1) |
                          ((p.7 & 1) << 0);
                i += 1;

                obuf[i] = ((p.0 & 2) << 6) |
                          ((p.1 & 2) << 5) |
                          ((p.2 & 2) << 4) |
                          ((p.3 & 2) << 3) |
                          ((p.4 & 2) << 2) |
                          ((p.5 & 2) << 1) |
                          ((p.6 & 2) << 0) |
                          ((p.7 & 2) >> 1);
                i += 1;

                if i >= 0x1000 {
                    return;
                }
            }
        }
    }
}
