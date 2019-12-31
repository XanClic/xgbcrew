use crate::io::{hdma_copy_16b, io_write};
use crate::io::int::IRQ;
use crate::system_state::{IOReg, SystemState};


pub struct DisplayState {
    sdl: sdl2::Sdl,
    evt_pump: Option<sdl2::EventPump>,
    cvs: Option<sdl2::render::Canvas<sdl2::video::Window>>,

    /*
     * This is unsafe.  AFAIU, the safe way to access the window's
     * surface would be to get the surface every time anything needs
     * to be drawn.  I don't want to do that, so, well, it stays
     * unsafe.
     */
    pixels: Option<*mut u8>,
    stride: isize,

    enabled: bool,
    wnd_tile_map: isize,
    wnd_enabled: bool,
    tile_data: isize,
    bg_tile_map: isize,
    obj_height: usize,
    obj_enabled: bool,
    tiles_enabled: bool,

    line_timer: u32,

    bcps: u8,
    ocps: u8,
    bg_palette: [u32; 32],
    bg_palette15: [u16; 32],
    obj_palette: [u32; 32],
    obj_palette15: [u16; 32],
}


impl DisplayState {
    pub fn new() -> Self {
        let mut me = Self {
            sdl: sdl2::init().unwrap(),
            evt_pump: None,
            cvs: None,
            pixels: None,
            stride: 0,

            enabled: false,
            wnd_tile_map: 0,
            wnd_enabled: false,
            tile_data: 0,
            bg_tile_map: 0,
            obj_height: 0,
            obj_enabled: false,
            tiles_enabled: false,

            line_timer: 0,

            bcps: 0,
            ocps: 0,
            bg_palette: [0xffffff; 32],
            bg_palette15: [0x7fff; 32],
            obj_palette: [0x00; 32],
            obj_palette15: [0x0000; 32],
        };

        let vid = me.sdl.video().unwrap();

        me.evt_pump = Some(me.sdl.event_pump().unwrap());
        let wnd = vid.window("xgbcrew", 160, 144).build().unwrap();

        let mut sfc = wnd.surface(me.evt_pump.as_ref().unwrap()).unwrap();
        me.stride = sfc.pitch() as isize;
        me.pixels = Some(sfc.without_lock_mut().unwrap().as_mut_ptr());

        me.cvs = Some(wnd.into_canvas().software().build().unwrap());

        me
    }

    pub fn init_system_state(sys_state: &mut SystemState) {
        io_write(sys_state, IOReg::LCDC as u16, 0x91);
        io_write(sys_state, IOReg::STAT as u16, 0x06);
        io_write(sys_state, IOReg::BGP  as u16, 0xfc);
        io_write(sys_state, IOReg::OBP0 as u16, 0xff);
        io_write(sys_state, IOReg::OBP1 as u16, 0xff);
    }

    fn update(&mut self) {
        self.cvs.as_mut().unwrap().present();
    }
}


fn draw_bg_line(sys_state: &SystemState, pixels: *mut u32,
                line: u8, bit7val: u8, window: bool)
{
    let d = &sys_state.display;

    let bg_tile_map = unsafe {
        (sys_state.addr_space.full_vram.offset(d.bg_tile_map),
         sys_state.addr_space.full_vram.offset(d.bg_tile_map + 0x2000))
    };

    let tile_data = unsafe {
        sys_state.addr_space.full_vram.offset(d.tile_data)
    };

    let tile_data_signed = d.tile_data == 0x1000;

    let sx = sys_state.io_regs[IOReg::SCX as usize];
    let mut bx = sx & 0xf8;
    let mut rsx = sx - bx;
    let ex = sx.wrapping_add(167) & 0xf8;
    let by = line & 0xf8;
    let ry = line & 0x07;
    let mut tile = ((by as isize) << 2) + ((bx as isize) >> 3);

    while bx != ex {
        if window && sys_state.io_regs[IOReg::WX as usize] <= bx {
            break;
        }

        let flags =
            if sys_state.cgb {
                unsafe {
                    *bg_tile_map.1.offset(tile)
                }
            } else {
                0
            };

        if flags & (1 << 7) != bit7val {
            let (nbx, wrap) = bx.overflowing_add(8);
            bx = nbx;
            if wrap {
                tile -= 31;
            } else {
                tile += 1;
            }
            rsx = 0;
            continue;
        }

        let (data_ofs, pal_bi) =
            if sys_state.cgb {
                (if flags & (1 << 3) != 0 { 0x2000 } else { 0 },
                 ((flags & 7) as usize) * 4)
            } else {
                (0, 0)
            };

        let data_ptr =
            unsafe {
                let map = *bg_tile_map.0.offset(tile);
                if tile_data_signed {
                    tile_data.offset(map as i8 as isize * 16 + data_ofs)
                } else {
                    tile_data.offset(map as isize * 16 + data_ofs)
                }
            };

        let data =
            unsafe {
                if flags & (1 << 6) == 0 {
                    (*data_ptr.offset(ry as isize * 2),
                     *data_ptr.offset(ry as isize * 2 + 1))
                } else {
                    (*data_ptr.offset((7 - ry as isize) * 2),
                     *data_ptr.offset((7 - ry as isize) * 2 + 1))
                }
            };

        for rx in rsx..8 {
            let screen_x = (bx + rx).wrapping_sub(sx) as isize;

            if screen_x >= 160 {
                break;
            }

            let mask =
                if flags & (1 << 5) == 0 {
                    1 << (7 - rx)
                } else {
                    1 << rx
                };

            let val = ((data.0 & mask != 0) as usize) |
                      (((data.1 & mask != 0) as usize) << 1);

            unsafe {
                *pixels.offset(screen_x) = d.bg_palette[pal_bi + val];
            }
        }

        let (nbx, wrap) = bx.overflowing_add(8);
        bx = nbx;
        if wrap {
            tile -= 31;
        } else {
            tile += 1;
        }
        rsx = 0;
    }
}


fn draw_line(sys_state: &SystemState, line: u8) {
    let d = &sys_state.display;

    let pixels = unsafe {
        d.pixels.unwrap().offset(line as isize * d.stride) as *mut u32
    };

    if !d.enabled {
        unsafe {
            libc::memset(pixels as *mut libc::c_void,
                         0xff, d.stride as usize);
        }
        return;
    }

    let sy = sys_state.io_regs[IOReg::SCY as usize];
    let abs_line = line.wrapping_add(sy);
    let window_active = d.wnd_enabled &&
                        sys_state.io_regs[IOReg::WX as usize] >= 7 &&
                        sys_state.io_regs[IOReg::WX as usize] <= 166 &&
                        sys_state.io_regs[IOReg::WY as usize] <= line;

    if !d.tiles_enabled {
        unsafe {
            libc::memset(pixels as *mut libc::c_void,
                         0x00, d.stride as usize);
        }
    } else {
        draw_bg_line(sys_state, pixels, abs_line, 0 << 7, window_active);
    }

    /* TODO: Window + OBJ */

    if d.tiles_enabled && sys_state.cgb {
        draw_bg_line(sys_state, pixels, abs_line, 1 << 7, window_active);
    }
}


fn stat_mode_transition(sys_state: &mut SystemState, ly: u8, from: u8, to: u8) {
    assert!((ly > 143) == (to == 1));

    let mut stat = sys_state.io_regs[IOReg::STAT as usize];

    stat = (stat & !7) | to;
    if ly == sys_state.io_regs[IOReg::LYC as usize] {
        stat |= 1 << 2;
    }

    sys_state.io_regs[IOReg::STAT as usize] = stat;
    sys_state.io_regs[IOReg::LY as usize] = ly;

    /* Care must be taken to only generate each interrupt on the
     * event's leading edge */
    if stat & 0b01000100 == 0b01000100 /* LYC match */ &&
       to == 2 || to == 1 /* First submodes per line */
    {
        sys_state.io_regs[IOReg::IF as usize] |= IRQ::LCDC as u8;
    }

    let new_mode = to != from || to == 1;

    if new_mode {
        if stat & 0b00100011 == 0b00100010 /* Mode 2 */ ||
           stat & 0b00010011 == 0b00010001 /* Mode 1 */ ||
           stat & 0b00001011 == 0b00001000 /* Mode 0 */
        {
            sys_state.io_regs[IOReg::IF as usize] |= IRQ::LCDC as u8;
        }

        if to == 1 {
            sys_state.io_regs[IOReg::IF as usize] |= IRQ::VBlank as u8;

            if from != 1 {
                /* Entered VBlank */
                sys_state.display.update();

                while let Some(evt) = sys_state.display.evt_pump
                                        .as_mut().unwrap().poll_event()
                {
                    match evt {
                        sdl2::event::Event::Quit { timestamp: _ } => {
                            std::process::exit(0);
                        },

                        _ => {},
                    }
                }
            }
        }
    }

    if to == 3 {
        draw_line(sys_state, ly);
    } else if to == 0 && sys_state.io_regs[IOReg::HDMA5 as usize] & 0x80 == 0 {
        hdma_copy_16b(sys_state);
    }
}

pub fn add_cycles(sys_state: &mut SystemState, mut cycles: u32) {
    if !sys_state.display.enabled {
        return;
    }

    if !sys_state.double_speed {
        cycles *= 2;
    }

    let mut line_timer = sys_state.display.line_timer + cycles;
    let mut ly = sys_state.io_regs[IOReg::LY as usize];

    loop {
        let submode = sys_state.io_regs[IOReg::STAT as usize] & 3;

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

const SHADE: [u32; 4] = [
    0xffffff,
    0xa8a8a8,
    0x505050,
    0x000000,
];

pub fn lcd_write(sys_state: &mut SystemState, addr: u16, mut val: u8) {
    match addr {
        0x40 => {
            let d = &mut sys_state.display;

            d.enabled       = val & (1 << 7) != 0;
            d.wnd_enabled   = val & (1 << 5) != 0;
            d.obj_enabled   = val & (1 << 1) != 0;
            d.tiles_enabled = val & (1 << 0) != 0;

            d.wnd_tile_map  = if val & (1 << 6) != 0 { 0x1c00 } else { 0x1800 };
            d.tile_data     = if val & (1 << 4) != 0 { 0x0000 } else { 0x1000 };
            d.bg_tile_map   = if val & (1 << 3) != 0 { 0x1c00 } else { 0x1800 };

            d.obj_height    = if val & (1 << 2) != 0 { 8 } else { 16 };
        },

        0x41 => {
            val = (sys_state.io_regs[addr as usize] & 0x87) | val & 0x78;
        },

        0x42 | 0x43 | 0x4a | 0x4b => (),

        0x47 => {
            if !sys_state.cgb {
                let d = &mut sys_state.display;

                d.bg_palette[0] = SHADE[(val as usize >> 0) & 0x3];
                d.bg_palette[1] = SHADE[(val as usize >> 2) & 0x3];
                d.bg_palette[2] = SHADE[(val as usize >> 4) & 0x3];
                d.bg_palette[3] = SHADE[(val as usize >> 6) & 0x3];
            }
        },

        0x48 => {
            if !sys_state.cgb {
                let d = &mut sys_state.display;

                d.obj_palette[0] = SHADE[(val as usize >> 0) & 0x3];
                d.obj_palette[1] = SHADE[(val as usize >> 2) & 0x3];
                d.obj_palette[2] = SHADE[(val as usize >> 4) & 0x3];
                d.obj_palette[3] = SHADE[(val as usize >> 6) & 0x3];
            }
        },

        0x49 => {
            if !sys_state.cgb {
                let d = &mut sys_state.display;

                d.obj_palette[4] = SHADE[(val as usize >> 0) & 0x3];
                d.obj_palette[5] = SHADE[(val as usize >> 2) & 0x3];
                d.obj_palette[6] = SHADE[(val as usize >> 4) & 0x3];
                d.obj_palette[7] = SHADE[(val as usize >> 6) & 0x3];
            }
        },

        0x68 => {
            let d = &mut sys_state.display;

            val &= 0xbf;

            let i = (val as usize & 0x3e) >> 1;
            sys_state.io_regs[IOReg::BCPD as usize] =
                if val & 0x01 == 0 {
                    d.bg_palette15[i] as u8
                } else {
                    (d.bg_palette15[i] >> 8) as u8
                };

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
            sys_state.io_regs[IOReg::OCPD as usize] =
                if val & 0x01 == 0 {
                    d.obj_palette15[i] as u8
                } else {
                    (d.obj_palette15[i] >> 8) as u8
                };

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
            // sys_state.io_regs[addr as usize] = val;
        }
    }

    sys_state.io_regs[addr as usize] = val;
}
