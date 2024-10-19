use crate::io::lcd::{DisplaySGBMask, rgb15_to_rgb24};
use crate::system_state::SystemState;


#[derive(Serialize, Deserialize, PartialEq)]
enum TransferDest {
    Palette,
    TilesLow,
    TilesHigh,
    BorderMapPalette,
    Oam,
    Sound,
    AttributeFiles,
    Data,
}

#[derive(SaveState)]
pub struct SGBState {
    packet_index: usize,
    packet_bit_index: usize,

    raw_packets: [[u8; 16]; 7],

    #[savestate(skip_if("version < 3"))]
    trn_dst: TransferDest,

    #[savestate(ref)]
    pal_data: [u8; 0x1000],
    #[savestate(skip_if("version < 3"), ref)]
    tiles: [u8; 0x2000],
    #[savestate(skip_if("version < 3"), ref)]
    border_map_palette: [u8; 0x1000],
    #[savestate(skip_if("version < 3"), ref)]
    attr_files: [u8; 0x1000],

    #[savestate(skip_if("version < 6"), ref,
                post_import("self.reload_border()"))]
    pub border_pixels: [u32; 256 * 224],

    /* Will always be set after a savestate is loaded */
    #[savestate(skip)]
    pub load_border: bool,

    #[savestate(skip_if("version < 7"))]
    border_enabled: bool,
}

impl SGBState {
    pub fn new() -> Self {
        Self {
            packet_index: 0,
            packet_bit_index: 0,

            raw_packets: [[0u8; 16]; 7],

            trn_dst: TransferDest::Data,

            pal_data: [0u8; 0x1000],
            tiles: [0u8; 0x2000],
            border_map_palette: [0u8; 0x1000],
            attr_files: [0u8; 0x1000],

            border_pixels: [0u32; 256 * 224],
            load_border: false,
            border_enabled: false,
        }
    }

    fn reload_border(&mut self) {
        if self.border_enabled {
            self.load_border = true;
        }
    }
}


fn sgb_palxy(sys_state: &mut SystemState, x: usize, y: usize) {
    let s = &sys_state.sgb_state;

    let col0 = s.raw_packets[0][1] as u16 |
             ((s.raw_packets[0][2] as u16) << 8);

    for p in 0..8 {
        sys_state.display.set_bg_pal(p * 4, col0);
    }

    for i in 0..6 {
        let col = s.raw_packets[0][i * 2 + 3] as u16 |
                ((s.raw_packets[0][i * 2 + 4] as u16) << 8);

        let p = if i < 3 { x } else { y };

        sys_state.display.set_bg_pal(p * 4 + (i % 3) + 1, col);
    }
}

fn sgb_attr_blk(sys_state: &mut SystemState) {
    let s = &mut sys_state.sgb_state;

    sys_state.display.sgb_attr_blk(0b001, 0, 0, 0, 19, 17);

    let mut i = 2;
    for _ in 0..s.raw_packets[0][1] {
        sys_state.display.sgb_attr_blk(
            s.raw_packets[i / 16][i % 16],
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

    if s.raw_packets[0][9] & 0x40 != 0 {
        sys_state.display.sgb_mask(DisplaySGBMask::NoMask);
    }

    if s.raw_packets[0][9] & 0x80 != 0 {
        let mut afi = (s.raw_packets[0][9] & 0x3f) as usize * (20 * 18);
        for x in (&mut sys_state.display.sgb_pal_bi) as &mut [u8] {
            *x = ((s.attr_files[afi / 4] >> (2 * (3 - afi % 4))) & 0x3) * 4;
            afi += 1;
        }
    }

    for pal_bi in 0..4 {
        let idx = s.raw_packets[0][pal_bi * 2 + 1] as usize |
                ((s.raw_packets[0][pal_bi * 2 + 2] as usize) << 8);

        for shade in 0..4 {
            let fpi = (idx * 4 + shade) * 2;
            let rgb15 = s.pal_data[fpi] as u16 | ((s.pal_data[fpi + 1] as u16) << 8);

            if shade == 0 && pal_bi == 0 {
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
    sys_state.sgb_state.trn_dst = TransferDest::Palette;
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

fn sgb_chr_trn(sys_state: &mut SystemState) {
    sys_state.sgb_state.trn_dst =
        if sys_state.sgb_state.raw_packets[0][1] & 0x1 == 0 {
            TransferDest::TilesLow
        } else {
            TransferDest::TilesHigh
        };

    sys_state.display.fill_for_sgb_buf = true;
}

fn sgb_pcr_trn(sys_state: &mut SystemState) {
    sys_state.sgb_state.trn_dst = TransferDest::BorderMapPalette;
    sys_state.display.fill_for_sgb_buf = true;
}

fn sgb_attr_trn(sys_state: &mut SystemState) {
    sys_state.sgb_state.trn_dst = TransferDest::AttributeFiles;
    sys_state.display.fill_for_sgb_buf = true;
}

fn sgb_mask_en(sys_state: &mut SystemState) {
    match sys_state.sgb_state.raw_packets[0][1] & 0x3 {
        0 => sys_state.display.sgb_mask(DisplaySGBMask::NoMask),
        1 => sys_state.display.sgb_mask(DisplaySGBMask::Freeze),
        2 => sys_state.display.sgb_mask(DisplaySGBMask::Black),
        3 => sys_state.display.sgb_mask(DisplaySGBMask::Color0),

        _ => unreachable!(),
    }
}

pub fn sgb_cmd(sys_state: &mut SystemState) {
    match sys_state.sgb_state.raw_packets[0][0] >> 3 {
        0x00 => sgb_palxy(sys_state, 0, 1),
        0x01 => sgb_palxy(sys_state, 2, 3),
        0x02 => sgb_palxy(sys_state, 0, 3),
        0x03 => sgb_palxy(sys_state, 1, 2),
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
        0x13 => sgb_chr_trn(sys_state),
        0x14 => sgb_pcr_trn(sys_state),
        0x15 => sgb_attr_trn(sys_state),
        0x16 => println!("SGB ATTR_SET unhandled"),
        0x17 => sgb_mask_en(sys_state),
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

fn sgb_buf_transfer(obuf: &mut [u8], ibuf: &[u8], start_i: usize) {
    let mut i = start_i;
    let end_i = start_i + 0x1000;

    for by in (0..144).step_by(8) {
        for x in (0..160).step_by(8) {
            for ry in 0..8 {
                let y = by + ry;

                let p = (ibuf[y * 160 + x],
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
                           (p.7 & 1);
                i += 1;

                obuf[i] = ((p.0 & 2) << 6) |
                          ((p.1 & 2) << 5) |
                          ((p.2 & 2) << 4) |
                          ((p.3 & 2) << 3) |
                          ((p.4 & 2) << 2) |
                          ((p.5 & 2) << 1) |
                           (p.6 & 2) |
                          ((p.7 & 2) >> 1);
                i += 1;

                if i >= end_i {
                    return;
                }
            }
        }
    }
}

pub fn sgb_buf_done(sys_state: &mut SystemState) {
    let s = &mut sys_state.sgb_state;
    let ibuf = &sys_state.display.for_sgb_buf;

    let i =
        match s.trn_dst {
            TransferDest::TilesHigh => 0x1000,
            _ => 0,
        };

    let obuf =
        match s.trn_dst {
            TransferDest::Palette => &mut s.pal_data as &mut [u8],
            TransferDest::TilesLow => &mut s.tiles as &mut [u8],
            TransferDest::TilesHigh => &mut s.tiles as &mut [u8],
            TransferDest::BorderMapPalette =>
                &mut s.border_map_palette as &mut [u8],
            TransferDest::AttributeFiles => &mut s.attr_files as &mut [u8],

            _ => unreachable!(),
        };

    sgb_buf_transfer(obuf, ibuf, i);

    if s.trn_dst == TransferDest::BorderMapPalette {
        sgb_construct_border_image(sys_state);
        sys_state.sgb_state.border_enabled = true;
        sys_state.sgb_state.reload_border();
    }
}

fn sgb_construct_border_image(sys_state: &mut SystemState) {
    let s = &mut sys_state.sgb_state;

    let mut border_pal = [0u32; 8 * 16];
    for pi in 0..(4 * 16) {
        let rgb15 =
            s.border_map_palette[0x800 + pi * 2] as u16 |
            ((s.border_map_palette[0x800 + pi * 2 + 1] as u16) << 8);

        border_pal[pi + 4 * 16] =
            if pi % 16 == 0 {
                sys_state.display.get_bg_pal(0)
            } else {
                rgb15_to_rgb24(rgb15)
            };
    }

    for pi in 0..4 {
        for shade in 0..16 {
            if shade < 4 {
                border_pal[pi * 16 + shade] =
                    sys_state.display.get_bg_pal(pi * 4 + shade);
            } else {
                border_pal[pi * 16 + shade] = 0xff00ff;
            }
        }
    }

    let mut i = 0;
    for y in 0..224 {
        for x in 0..256 {
            let map_i = (y / 8) * 32 + x / 8;
            let map =
                s.border_map_palette[map_i * 2] as usize |
                ((s.border_map_palette[map_i * 2 + 1] as usize) << 8);

            let tile_i = map & 0xff;
            let pal_bi = ((map >> 10) & 0x7) * 16;

            let xm =
                if map & (1 << 14) == 0 {
                    7 - (x % 8)
                } else {
                    x % 8
                };

            let ym =
                if map & (1 << 15) == 0 {
                    y % 8
                } else {
                    7 - (y % 8)
                };

            let shade_planed =
                ((s.tiles[tile_i * 32 + ym * 2] >> xm) & 0x1,
                 (s.tiles[tile_i * 32 + ym * 2 + 1] >> xm) & 0x1,
                 (s.tiles[tile_i * 32 + ym * 2 + 16] >> xm) & 0x1,
                 (s.tiles[tile_i * 32 + ym * 2 + 17] >> xm) & 0x1);

            let shade = shade_planed.0 |
                       (shade_planed.1 << 1) |
                       (shade_planed.2 << 2) |
                       (shade_planed.3 << 3);

            s.border_pixels[i] = border_pal[pal_bi + shade as usize];
            i += 1;
        }
    }
}
