use crate::address_space::get_raw_read_addr;
use crate::io::{hdma_copy_16b, io_get_reg, io_set_addr, io_set_reg, io_write};
use crate::io::int::IRQ;
use crate::sgb::sgb_buf_done;
use crate::system_state::{IOReg, SystemState};


#[derive(Serialize, Deserialize, PartialEq)]
pub enum DisplaySGBMask {
    NoMask,
    Freeze,
    Black,
    Color0,
}

#[derive(SaveState)]
pub struct DisplayState {
    #[savestate(skip_if("version < 2"),
                import_fn("savestate::import_u32_slice"),
                export_fn("savestate::export_u32_slice"))]
    pub lcd_pixels: [u32; 160 * 144],

    enabled: bool,
    wnd_tile_map: isize,
    wnd_enabled: bool,
    tile_data: isize,
    bg_tile_map: isize,
    obj_height: isize,
    obj_enabled: bool,
    bg_enabled: bool,
    obj_prio: bool,

    line_timer: u32,

    bcps: u8,
    ocps: u8,
    bg_palette: [u32; 32],
    bg_palette15: [u16; 32],
    obj_palette: [u32; 32],
    obj_palette15: [u16; 32],

    #[savestate(skip_if("version < 1"))]
    bg_palette_mapping: [u8; 4],
    #[savestate(skip_if("version < 1"))]
    obj_palette_mapping: [u8; 8],

    #[savestate(skip_if("version < 1"),
                import_fn("savestate::import_u8_slice"),
                export_fn("savestate::export_u8_slice"))]
    sgb_pal_bi: [u8; 20 * 18],

    #[savestate(skip_if("version < 1"),
                import_fn("savestate::import_u8_slice"),
                export_fn("savestate::export_u8_slice"))]
    pub for_sgb_buf: [u8; 160 * 144],
    #[savestate(skip_if("version < 1"))]
    pub fill_for_sgb_buf: bool,
    #[savestate(skip_if("version < 1"))]
    pub filling_for_sgb_buf: bool,

    #[savestate(skip_if("version < 2"))]
    sgb_mask: DisplaySGBMask,
    #[savestate(skip_if("version < 2"),
                import_fn("savestate::import_u32_slice"),
                export_fn("savestate::export_u32_slice"))]
    sgb_freeze: [u32; 160 * 144],
}


impl DisplayState {
    pub fn new() -> Self {
        Self {
            lcd_pixels: [0; 160 * 144],

            enabled: false,
            wnd_tile_map: 0,
            wnd_enabled: false,
            tile_data: 0,
            bg_tile_map: 0,
            obj_height: 0,
            obj_enabled: false,
            bg_enabled: false,
            obj_prio: false,

            line_timer: 0,

            bcps: 0,
            ocps: 0,

            bg_palette: [0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                         0xffffff, 0xa8a8a8, 0x505050, 0x000000],

            bg_palette15: [0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000,
                           0x7fff, 0x5294, 0x294a, 0x0000],

            obj_palette: [0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000,
                          0xffffff, 0xa8a8a8, 0x505050, 0x000000],

            obj_palette15: [0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000,
                            0x7fff, 0x5294, 0x294a, 0x0000],

            bg_palette_mapping: [0, 1, 2, 3],
            obj_palette_mapping: [0, 1, 2, 3, 4, 5, 6, 7],

            sgb_pal_bi: [0u8; 20 * 18],

            for_sgb_buf: [0u8; 160 * 144],
            fill_for_sgb_buf: false,
            filling_for_sgb_buf: false,

            sgb_mask: DisplaySGBMask::NoMask,
            sgb_freeze: [0u32; 160 * 144],
        }
    }

    pub fn init_system_state(sys_state: &mut SystemState) {
        io_write(sys_state, IOReg::LCDC as u16, 0x91);
        io_write(sys_state, IOReg::STAT as u16, 0x06);
        io_write(sys_state, IOReg::BGP  as u16, 0xfc);
        io_write(sys_state, IOReg::OBP0 as u16, 0xff);
        io_write(sys_state, IOReg::OBP1 as u16, 0xff);
    }

    pub fn set_bg_pal(&mut self, index: usize, rgb15: u16) {
        self.bg_palette[index] = rgb15_to_rgb24(rgb15);
    }

    pub fn set_obj_pal(&mut self, index: usize, rgb15: u16) {
        self.obj_palette[index] = rgb15_to_rgb24(rgb15);
    }

    pub fn sgb_attr_blk(&mut self, ctrl: u8, pal: u8,
                        x1: usize, y1: usize, x2: usize, y2: usize)
    {
        let (inner_s, outer_s, border_s) =
            match ctrl & 0b111 {
                0b000 => (None,    None,    None),
                0b001 => (Some(0), None,    Some(0)),
                0b010 => (None,    None,    Some(2)),
                0b011 => (Some(0), None,    Some(2)),
                0b100 => (None,    Some(4), Some(4)),
                0b101 => (Some(0), Some(4), None),
                0b110 => (None,    Some(4), Some(2)),
                0b111 => (Some(0), Some(4), Some(2)),

                _ => unreachable!(),
            };

        let inner =
            if let Some(s) = inner_s {
                Some(((pal >> s) & 0x3) * 4)
            } else {
                None
            };

        let outer =
            if let Some(s) = outer_s {
                Some(((pal >> s) & 0x3) * 4)
            } else {
                None
            };

        let border =
            if let Some(s) = border_s {
                Some(((pal >> s) & 0x3) * 4)
            } else {
                None
            };

        let mut i = 0;
        for y in 0..18 {
            for x in 0..20 {
                if x < x1 || x > x2 || y < y1 || y > y2 {
                    if let Some(p) = outer {
                        self.sgb_pal_bi[i] = p;
                    }
                } else if x > x1 && x < x2 && y > y1 && y < y2 {
                    if let Some(p) = inner {
                        self.sgb_pal_bi[i] = p;
                    }
                } else {
                    if let Some(p) = border {
                        self.sgb_pal_bi[i] = p;
                    }
                }

                i += 1;
            }
        }
    }

    pub fn sgb_mask(&mut self, mask: DisplaySGBMask) {
        if mask == DisplaySGBMask::Freeze {
            /* FIXME: Should use an old complete frame */
            for i in 0..(160 * 144) {
                self.sgb_freeze[i] = self.lcd_pixels[i];
            }
        }

        self.sgb_mask = mask;
    }
}


fn fetch_tile_flags(map: (*const u8, *const u8), tile: isize, cgb: bool) -> u8 {
    if cgb {
        unsafe {
            *map.1.offset(tile)
        }
    } else {
        0
    }
}

fn fetch_tile_obj_data(data_ptr: *const u8, flags: u8, ry: isize, height: isize)
    -> (u8, u8)
{
    unsafe {
        if flags & (1 << 6) == 0 {
            (*data_ptr.offset(ry * 2 + 0),
             *data_ptr.offset(ry * 2 + 1))
        } else {
            (*data_ptr.offset((height - 1 - ry) * 2 + 0),
             *data_ptr.offset((height - 1 - ry) * 2 + 1))
        }
    }
}

fn get_tile_data_and_pal(map: (*const u8, *const u8), tile_data: *const u8,
                         tile_data_signed: bool, flags: u8, tile: isize,
                         ry: isize, height: isize, cgb: bool)
    -> ((u8, u8), usize)
{
    let (data_ofs, pal_bi) =
        if cgb {
            (if flags & (1 << 3) != 0 { 0x2000 } else { 0 },
             ((flags & 7) as usize) * 4)
        } else {
            (0, 0)
        };

    let data_ptr =
        unsafe {
            let mapping = *map.0.offset(tile);
            if tile_data_signed {
                tile_data.offset(mapping as i8 as isize * 16 + data_ofs)
            } else {
                tile_data.offset(mapping as isize * 16 + data_ofs)
            }
        };

    (fetch_tile_obj_data(data_ptr, flags, ry, height), pal_bi)
}

fn get_tile_obj_pixel(data: (u8, u8), rx: u8, flags: u8) -> usize {
    let mask =
        if flags & (1 << 5) == 0 {
            1 << (7 - rx)
        } else {
            1 << rx
        };

    ((data.0 & mask != 0) as usize) |
    (((data.1 & mask != 0) as usize) << 1)
}

fn get_tile_prio(pixel: usize, flags: u8, obj_prio: bool) -> u8 {
    if obj_prio || pixel == 0 {
        0
    } else if flags & (1 << 7) != 0 {
        2
    } else {
        1
    }
}


fn draw_bg_line(sys_state: &mut SystemState,
                line: u8, screen_line: u8, window: bool,
                bg_prio: &mut [u8; 160])
{
    let d = &mut sys_state.display;
    let sofs = screen_line as usize * 160;
    let eofs = sofs + 160;
    let pixels = &mut d.lcd_pixels[sofs..eofs];

    let bg_tile_map = unsafe {
        (sys_state.addr_space.full_vram.offset(d.bg_tile_map) as *const u8,
         sys_state.addr_space.full_vram.offset(d.bg_tile_map + 0x2000)
             as *const u8)
    };

    let tile_data = unsafe {
        sys_state.addr_space.full_vram.offset(d.tile_data) as *const u8
    };

    let tile_data_signed = d.tile_data == 0x1000;

    let sx = io_get_reg(IOReg::SCX);
    let wx = io_get_reg(IOReg::WX).wrapping_sub(7);
    let mut bx = sx & 0xf8;
    let ex = sx.wrapping_add(167) & 0xf8;
    let by = (line & 0xf8) as isize;
    let ry = (line & 0x07) as isize;
    let mut tile = (by << 2) + ((bx as isize) >> 3);

    while bx != ex {
        if window && wx <= bx {
            break;
        }

        let flags = fetch_tile_flags(bg_tile_map, tile, sys_state.cgb);
        let (data, mut pal_bi) = get_tile_data_and_pal(bg_tile_map, tile_data,
                                                       tile_data_signed, flags,
                                                       tile, ry, 8,
                                                       sys_state.cgb);

        for rx in 0..8 {
            let screen_x = (bx + rx).wrapping_sub(sx) as usize;
            if screen_x >= 160 {
                continue;
            }

            if sys_state.sgb {
                pal_bi = d.sgb_pal_bi[(screen_line as usize / 8) * 20 +
                                      screen_x as usize / 8] as usize;
            }

            let val = get_tile_obj_pixel(data, rx, flags);
            let pal_i = d.bg_palette_mapping[val] as usize;
            pixels[screen_x] = d.bg_palette[pal_bi + pal_i];
            bg_prio[screen_x] = get_tile_prio(val, flags, d.obj_prio);

            if d.filling_for_sgb_buf {
                d.for_sgb_buf[screen_line as usize * 160 + screen_x as usize] =
                    pal_i as u8;
            }
        }

        let (nbx, wrap) = bx.overflowing_add(8);
        bx = nbx;
        if wrap {
            tile -= 31;
        } else {
            tile += 1;
        }
    }
}


fn draw_wnd_line(sys_state: &mut SystemState,
                 screen_line: u8, bg_prio: &mut [u8; 160])
{
    let d = &mut sys_state.display;
    let sofs = screen_line as usize * 160;
    let eofs = sofs + 160;
    let pixels = &mut d.lcd_pixels[sofs..eofs];

    let wx = io_get_reg(IOReg::WX) - 7;
    let wy = io_get_reg(IOReg::WY);

    if screen_line < wy {
        return;
    }

    let by = (screen_line - wy) & 0xf8;
    let ry = (screen_line - wy) & 0x07;

    let wnd_tile_map = unsafe {
        (sys_state.addr_space.full_vram.offset(d.wnd_tile_map) as *const u8,
         sys_state.addr_space.full_vram.offset(d.wnd_tile_map + 0x2000)
             as *const u8)
    };

    let tile_data = unsafe {
        sys_state.addr_space.full_vram.offset(d.tile_data) as *const u8
    };

    let tile_data_signed = d.tile_data == 0x1000;

    let mut tile = (by as isize) << 2;

    for bx in (wx..160).step_by(8) {
        let flags = fetch_tile_flags(wnd_tile_map, tile, sys_state.cgb);
        let (data, mut pal_bi) = get_tile_data_and_pal(wnd_tile_map, tile_data,
                                                       tile_data_signed, flags,
                                                       tile, ry as isize, 8,
                                                       sys_state.cgb);

        for rx in 0..8 {
            let screen_x = (bx + rx) as usize;
            if screen_x >= 160 {
                break;
            }

            if sys_state.sgb {
                pal_bi = d.sgb_pal_bi[(screen_line as usize / 8) * 20 +
                                      screen_x as usize / 8] as usize;
            }

            let val = get_tile_obj_pixel(data, rx, flags);
            let pal_i = d.bg_palette_mapping[val] as usize;
            pixels[screen_x] = d.bg_palette[pal_bi + pal_i];
            bg_prio[screen_x] = get_tile_prio(val, flags, d.obj_prio);

            if d.filling_for_sgb_buf {
                d.for_sgb_buf[screen_line as usize * 160 + screen_x as usize] =
                    pal_i as u8;
            }
        }

        tile += 1;
    }
}


fn draw_obj_line(sys_state: &mut SystemState, screen_line: u8,
                 bg_prio: &[u8; 160])
{
    let d = &mut sys_state.display;
    let sofs = screen_line as usize * 160;
    let eofs = sofs + 160;
    let pixels = &mut d.lcd_pixels[sofs..eofs];
    let oam = get_raw_read_addr(0xfe00) as *const u8;

    let mut count = 0;

    /* TODO: Priority should be given to objects at lower X */
    for sprite in (0..40).rev() {
        let oam_bi = sprite * 4;
        let by = unsafe { *oam.offset(oam_bi + 0) } as isize - 16;
        let bx = unsafe { *oam.offset(oam_bi + 1) } as isize - 8;

        if by > screen_line as isize ||
           by + d.obj_height <= screen_line as isize
        {
            continue;
        }

        count += 1;
        if count > 10 {
            break;
        }

        if bx <= -8 || bx >= 160 {
            continue;
        }

        let mut ofs = unsafe { *oam.offset(oam_bi + 2) } as isize * 16;
        let flags = unsafe { *oam.offset(oam_bi + 3) };

        if d.obj_height == 16 {
            ofs &= !0x1f;
        }

        let (data_ofs, mut pal_bi) =
            if sys_state.cgb {
                (if flags & (1 << 3) != 0 { 0x2000 } else { 0 },
                 ((flags & 7) as usize) * 4)
            } else {
                (0,
                 ((flags >> 4) & 1) as usize)
            };

        let data_ptr =
            unsafe {
                sys_state.addr_space.full_vram.offset(data_ofs + ofs)
            };

        let data = fetch_tile_obj_data(data_ptr, flags,
                                       screen_line as isize - by,
                                       d.obj_height);

        for rx in 0..8 {
            let screen_x = (bx + rx) as usize;
            if screen_x >= 160 {
                continue;
            }

            if sys_state.sgb {
                pal_bi = d.sgb_pal_bi[(screen_line as usize / 8) * 20 +
                                      screen_x as usize / 8] as usize;
            }

            let val = get_tile_obj_pixel(data, rx as u8, flags);
            if val != 0 && bg_prio[screen_x] < 2 {
                if flags & (1 << 7) == 0 || bg_prio[screen_x] < 1 {
                    let pal_i = d.obj_palette_mapping[val] as usize;
                    pixels[screen_x] = d.obj_palette[pal_bi + pal_i];

                    if d.filling_for_sgb_buf {
                        d.for_sgb_buf[screen_line as usize * 160 +
                                      screen_x as usize] = pal_i as u8;
                    }
                }
            }
        }
    }
}


fn draw_line(sys_state: &mut SystemState, line: u8) {
    let sofs = line as usize * 160;
    let eofs = sofs + 160;
    let pixels = &mut sys_state.display.lcd_pixels[sofs..eofs];
    let mut bg_prio = [0u8; 160];

    if !sys_state.display.enabled {
        for p in pixels {
            *p = 0xffffff;
        }
        return;
    }

    let sy = io_get_reg(IOReg::SCY);
    let abs_line = line.wrapping_add(sy);
    let wx = io_get_reg(IOReg::WX);
    let wy = io_get_reg(IOReg::WY);
    let window_active = sys_state.display.wnd_enabled &&
                        wx >= 7 && wx <= 166 && wy <= line;

    if !sys_state.display.bg_enabled {
        for p in pixels {
            *p = 0x000000;
        }
    } else {
        draw_bg_line(sys_state, abs_line, line, window_active, &mut bg_prio);
    }

    if window_active {
        draw_wnd_line(sys_state, line, &mut bg_prio);
    }

    if sys_state.display.obj_enabled {
        draw_obj_line(sys_state, line, &bg_prio);
    }
}


fn stat_mode_transition(sys_state: &mut SystemState, ly: u8, from: u8, to: u8) {
    let d = &mut sys_state.display;

    assert!((ly > 143) == (to == 1));

    let mut stat = io_get_reg(IOReg::STAT);

    stat = (stat & !7) | to;
    if ly == io_get_reg(IOReg::LYC) {
        stat |= 1 << 2;
    }

    io_set_reg(IOReg::STAT, stat);
    io_set_reg(IOReg::LY, ly);

    /* Care must be taken to only generate each interrupt on the
     * event's leading edge */
    if stat & 0b01000100 == 0b01000100 /* LYC match */ &&
       (to == 2 || to == 1) /* First submodes per line */
    {
        io_set_reg(IOReg::IF, io_get_reg(IOReg::IF) | (IRQ::LCDC as u8));
    }

    if to != from {
        if stat & 0b00100011 == 0b00100010 /* Mode 2 */ ||
           stat & 0b00010011 == 0b00010001 /* Mode 1 */ ||
           stat & 0b00001011 == 0b00001000 /* Mode 0 */
        {
            io_set_reg(IOReg::IF, io_get_reg(IOReg::IF) | (IRQ::LCDC as u8));
        }

        if to == 1 {
            /* Entered VBlank */
            io_set_reg(IOReg::IF, io_get_reg(IOReg::IF) | (IRQ::VBlank as u8));

            sys_state.vblanked = true;

            match d.sgb_mask {
                DisplaySGBMask::NoMask => (),

                DisplaySGBMask::Freeze => {
                    for i in 0..(160 * 144) {
                        d.lcd_pixels[i] = d.sgb_freeze[i];
                    }
                },

                DisplaySGBMask::Black => {
                    for i in 0..(160 * 144) {
                        d.lcd_pixels[i] = 0x000000;
                    }
                },

                DisplaySGBMask::Color0 => {
                    for i in 0..(160 * 144) {
                        d.lcd_pixels[i] = d.bg_palette[0];
                    }
                },
            }

            if d.filling_for_sgb_buf {
                d.filling_for_sgb_buf = false;
                sgb_buf_done(sys_state);
            }

            if sys_state.display.fill_for_sgb_buf {
                sys_state.display.fill_for_sgb_buf = false;
                sys_state.display.filling_for_sgb_buf = true;
            }
        }
    }

    if to == 3 {
        draw_line(sys_state, ly);
    } else if to == 0 && io_get_reg(IOReg::HDMA5) & 0x80 == 0 {
        hdma_copy_16b(sys_state);
    }
}

/* @cycles must be in double-speed cycles */
pub fn add_cycles(sys_state: &mut SystemState, cycles: u32) {
    if !sys_state.display.enabled {
        return;
    }

    let mut line_timer = sys_state.display.line_timer + cycles;
    let mut ly = io_get_reg(IOReg::LY);

    loop {
        let submode = io_get_reg(IOReg::STAT) & 3;

        if submode == 1 {
            if line_timer >= 228 {
                ly += 1;
                if ly < 154 {
                    /* VBlank -> VBlank */
                    stat_mode_transition(sys_state, ly, submode, 1);
                } else {
                    /* VBlank -> OAM-only */
                    ly = 0;
                    stat_mode_transition(sys_state, ly, submode, 2);
                }
                line_timer -= 228;
            } else {
                break;
            }
        } else if submode == 2 {
            if line_timer >= 40 {
                /* OAM-only -> OAM+VRAM */
                stat_mode_transition(sys_state, ly, submode, 3);
                line_timer -= 40;
            } else {
                break;
            }
        } else if submode == 3 {
            if line_timer >= 86 {
                /* OAM+VRAM -> HBlank */
                stat_mode_transition(sys_state, ly, submode, 0);
                line_timer -= 86;
            } else {
                break;
            }
        } else /* if submode == 4 */ {
            if line_timer >= 102 {
                ly += 1;
                if ly < 144 {
                    /* HBlank -> OAM-only */
                    stat_mode_transition(sys_state, ly, submode, 2);
                } else {
                    /* HBlank -> VBlank */
                    stat_mode_transition(sys_state, ly, submode, 1);
                }
                line_timer -= 102;
            } else {
                break;
            }
        }
    }

    sys_state.display.line_timer = line_timer;
}


/* TODO: Implement better translation function */
fn rgb15_to_rgb24(rgb15: u16) -> u32 {
    let r =  rgb15        & 0x1f;
    let g = (rgb15 >>  5) & 0x1f;
    let b = (rgb15 >> 10) & 0x1f;

    let r8 = ((r * 255) / 31) as u32;
    let g8 = ((g * 255) / 31) as u32;
    let b8 = ((b * 255) / 31) as u32;

    (r8 << 16) | (g8 << 8) | b8
}

pub fn lcd_write(sys_state: &mut SystemState, addr: u16, mut val: u8) {
    match addr {
        0x40 => {
            let d = &mut sys_state.display;

            d.enabled       = val & (1 << 7) != 0;
            d.wnd_enabled   = val & (1 << 5) != 0;
            d.obj_enabled   = val & (1 << 1) != 0;

            if sys_state.cgb {
                /* XXX: One doc says this, but the official doc mentions
                 * nothing of this sort */
                /* d.obj_prio   = val & (1 << 0) != 0; */
                d.obj_prio   = false;
                d.bg_enabled = true;
            } else {
                d.bg_enabled = val & (1 << 0) != 0;
                d.obj_prio   = false;
            }

            d.wnd_tile_map  = if val & (1 << 6) != 0 { 0x1c00 } else { 0x1800 };
            d.tile_data     = if val & (1 << 4) != 0 { 0x0000 } else { 0x1000 };
            d.bg_tile_map   = if val & (1 << 3) != 0 { 0x1c00 } else { 0x1800 };

            d.obj_height    = if val & (1 << 2) != 0 { 16 } else { 8 };

            if !d.enabled {
                stat_mode_transition(sys_state, 0, io_get_reg(IOReg::STAT) & 3,
                                     0);
            }
        },

        0x41 => {
            val = (io_get_reg(IOReg::STAT) & 0x87) | val & 0x78;
        },

        0x42 | 0x43 | 0x4a | 0x4b => (),

        0x44 => {
            val = 0;
        },

        0x45 => {
            if val == io_get_reg(IOReg::LY) {
                let mut stat = io_get_reg(IOReg::STAT);
                stat |= 1 << 2;
                io_set_reg(IOReg::STAT, stat);

                if stat & 0b01000100 == 0b01000100 {
                    io_set_reg(IOReg::IF,
                               io_get_reg(IOReg::IF) | (IRQ::LCDC as u8));
                }
            }
        },

        0x47 => {
            if !sys_state.cgb {
                let d = &mut sys_state.display;

                d.bg_palette_mapping[0] = (val >> 0) & 0x3;
                d.bg_palette_mapping[1] = (val >> 2) & 0x3;
                d.bg_palette_mapping[2] = (val >> 4) & 0x3;
                d.bg_palette_mapping[3] = (val >> 6) & 0x3;
            }
        },

        0x48 => {
            if !sys_state.cgb {
                let d = &mut sys_state.display;

                d.obj_palette_mapping[0] = (val >> 0) & 0x3;
                d.obj_palette_mapping[1] = (val >> 2) & 0x3;
                d.obj_palette_mapping[2] = (val >> 4) & 0x3;
                d.obj_palette_mapping[3] = (val >> 6) & 0x3;
            }
        },

        0x49 => {
            if !sys_state.cgb {
                let d = &mut sys_state.display;

                d.obj_palette_mapping[4] = (val >> 0) & 0x3;
                d.obj_palette_mapping[5] = (val >> 2) & 0x3;
                d.obj_palette_mapping[6] = (val >> 4) & 0x3;
                d.obj_palette_mapping[7] = (val >> 6) & 0x3;
            }
        },

        0x68 => {
            let d = &mut sys_state.display;

            val &= 0xbf;

            let i = (val as usize & 0x3e) >> 1;
            io_set_reg(IOReg::BCPD,
                       if val & 0x01 == 0 {
                           d.bg_palette15[i] as u8
                       } else {
                           (d.bg_palette15[i] >> 8) as u8
                       });

            d.bcps = val;
        },

        0x69 => {
            let bcps = {
                let d = &mut sys_state.display;

                let i = (d.bcps as usize & 0x3e) >> 1;
                if d.bcps & 0x01 == 0 {
                    d.bg_palette15[i] =
                        (d.bg_palette15[i] & 0xff00) |
                        (val as u16);
                } else {
                    val &= 0x7f;
                    d.bg_palette15[i] =
                        (d.bg_palette15[i] & 0x00ff) |
                        ((val as u16) << 8);
                }

                d.bg_palette[i] = rgb15_to_rgb24(d.bg_palette15[i]);

                d.bcps
            };

            if bcps & 0x80 != 0 {
                lcd_write(sys_state, IOReg::BCPS as u16, (bcps + 1) & 0xbf);
            }
        },

        0x6a => {
            let d = &mut sys_state.display;

            val &= 0xbf;

            let i = (val as usize & 0x3e) >> 1;
            io_set_reg(IOReg::OCPD,
                       if val & 0x01 == 0 {
                           d.obj_palette15[i] as u8
                       } else {
                           (d.obj_palette15[i] >> 8) as u8
                       });

            d.ocps = val;
        },

        0x6b => {
            let ocps = {
                let d = &mut sys_state.display;

                let i = (d.ocps as usize & 0x3e) >> 1;
                if d.ocps & 0x01 == 0 {
                    d.obj_palette15[i] =
                        (d.obj_palette15[i] & 0xff00) |
                        (val as u16);
                } else {
                    val &= 0x7f;
                    d.obj_palette15[i] =
                        (d.obj_palette15[i] & 0x00ff) |
                        ((val as u16) << 8);
                }

                d.obj_palette[i] = rgb15_to_rgb24(d.obj_palette15[i]);

                d.ocps
            };

            if ocps & 0x80 != 0 {
                lcd_write(sys_state, IOReg::OCPS as u16, (ocps + 1) & 0xbf);
            }
        },

        _ => {
            panic!("Unknown LCD register 0xff{:02x} (w 0x{:02x})", addr, val);
            // io_set_reg(addr, val);
        }
    }

    io_set_addr(addr, val);
}
