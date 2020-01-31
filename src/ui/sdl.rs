use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::ui::{AudioOutputParams, UIEvent, UIScancode};
use crate::ui::sc::SC;


pub struct SDLUI {
    sdl_audio: sdl2::AudioSubsystem,
    sdl_evt_pump: sdl2::EventPump,

    wnd_cvs: sdl2::render::Canvas<sdl2::video::Window>,
    lcd_txt: sdl2::render::Texture<'static>,
    lcd_rect: sdl2::rect::Rect,
    sgb_border: bool,
    sgb_border_txt: sdl2::render::Texture<'static>,
    border_rect: sdl2::rect::Rect,
    full_screen_update_counter: u32,

    audio_dev: Option<sdl2::audio::AudioDevice<AudioOutput>>,

    sc: Option<SC>,
}

impl SDLUI {
    pub fn new() -> Self {
        let sdl = sdl2::init().unwrap();

        let video = sdl.video().unwrap();
        let audio = sdl.audio().unwrap();
        let evt_pump = sdl.event_pump().unwrap();

        let wnd = video.window("xgbcrew", 160, 144).opengl().resizable().build()
                       .unwrap();
        let cvs = wnd.into_canvas().accelerated().build().unwrap();
        let txtc = cvs.texture_creator();

        let pixel_fmt = sdl2::pixels::PixelFormatEnum::ARGB8888;
        let access = sdl2::render::TextureAccess::Streaming;
        let lcd_txt = unsafe {
            /* F this */
            std::mem::transmute::<sdl2::render::Texture,
                                  sdl2::render::Texture<'static>>(
                txtc.create_texture(pixel_fmt, access, 160, 144).unwrap()
            )
        };

        let sgb_border_txt = unsafe {
            std::mem::transmute::<sdl2::render::Texture,
                                  sdl2::render::Texture<'static>>(
                txtc.create_texture(pixel_fmt, access, 256, 224).unwrap()
            )
        };

        Self {
            sdl_audio: audio,
            sdl_evt_pump: evt_pump,

            wnd_cvs: cvs,
            lcd_txt: lcd_txt,
            lcd_rect: sdl2::rect::Rect::new(0, 0, 160, 144),
            sgb_border: false,
            sgb_border_txt: sgb_border_txt,
            border_rect: sdl2::rect::Rect::new(0, 0, 160, 144),
            full_screen_update_counter: 0,

            audio_dev: None,

            sc: SC::new(),
        }
    }

    pub fn setup_audio(&mut self, params: AudioOutputParams) {
        let sound_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(params.freq as i32),
            channels: Some(params.channels as u8),
            samples: Some((params.buf_step / params.channels) as u16),
        };

        let adev_obj_gen = |_| {
            AudioOutput {
                buf: params.buf,
                buf_i: 0,
                buf_done: params.buf_done,
            }
        };

        let adev = self.sdl_audio.open_playback(None, &sound_spec,
                                                adev_obj_gen).unwrap();
        adev.resume();

        self.audio_dev = Some(adev);
    }

    pub fn present_frame(&mut self, pixels: &[u32; 160 * 144]) {
        let pixels8 = unsafe {
            std::slice::from_raw_parts(pixels.as_ptr() as *const u8,
                                       pixels.len() * 4)
        };

        self.full_screen_update_counter += 1;
        if self.full_screen_update_counter == 30 {
            self.update_bg();
            self.full_screen_update_counter = 0;
        }

        self.lcd_txt.update(None, pixels8, 160 * 4).unwrap();
        self.wnd_cvs.copy(&self.lcd_txt, None, Some(self.lcd_rect)).unwrap();
        self.wnd_cvs.present();
    }

    fn update_bg(&mut self) {
        self.wnd_cvs.clear();

        if self.sgb_border {
            self.wnd_cvs.copy(&self.sgb_border_txt, None,
                              Some(self.border_rect)).unwrap();
        }
    }

    fn update_rects(&mut self, w: u32, h: u32) {
        let (raw_w, raw_h) =
            if self.sgb_border {
                (256, 224)
            } else {
                (160, 144)
            };

        let (aspect_w, aspect_h) =
            if h * raw_w / raw_h < w {
                (h * raw_w / raw_h, h)
            } else {
                (w, w * raw_h / raw_w)
            };

        let (lcd_w, lcd_h) = (aspect_w * 160 / raw_w, aspect_h * 144 / raw_h);
        let (border_w, border_h) = (aspect_w, aspect_h);

        let center = sdl2::rect::Point::new(w as i32 / 2, h as i32 / 2);

        self.lcd_rect = sdl2::rect::Rect::from_center(center, lcd_w, lcd_h);
        self.border_rect = sdl2::rect::Rect::from_center(center,
                                                         border_w, border_h);

        self.update_bg();
    }

    fn sdl_sc_to_ui_sc(sdl_sc: sdl2::keyboard::Scancode) -> Option<UIScancode> {
        use sdl2::keyboard::Scancode;

        let ui_sc =
            match sdl_sc {
                Scancode::P         => UIScancode::P,
                Scancode::X         => UIScancode::X,
                Scancode::Z         => UIScancode::Z,

                Scancode::LShift    => UIScancode::Shift,
                Scancode::RShift    => UIScancode::Shift,
                Scancode::LAlt      => UIScancode::Alt,
                Scancode::RAlt      => UIScancode::Alt,
                Scancode::LCtrl     => UIScancode::Control,
                Scancode::RCtrl     => UIScancode::Control,

                Scancode::Space     => UIScancode::Space,
                Scancode::Return    => UIScancode::Return,
                Scancode::Backspace => UIScancode::Backspace,

                Scancode::Left      => UIScancode::Left,
                Scancode::Right     => UIScancode::Right,
                Scancode::Up        => UIScancode::Up,
                Scancode::Down      => UIScancode::Down,

                Scancode::F1        => UIScancode::F1,
                Scancode::F2        => UIScancode::F2,
                Scancode::F3        => UIScancode::F3,
                Scancode::F4        => UIScancode::F4,
                Scancode::F5        => UIScancode::F5,
                Scancode::F6        => UIScancode::F6,
                Scancode::F7        => UIScancode::F7,
                Scancode::F8        => UIScancode::F8,
                Scancode::F9        => UIScancode::F9,
                Scancode::F10       => UIScancode::F10,
                Scancode::F11       => UIScancode::F11,
                Scancode::F12       => UIScancode::F12,

                _ => { return None; },
            };

        Some(ui_sc)
    }

    fn translate_event(&mut self, evt: sdl2::event::Event) -> Option<UIEvent> {
        match evt {
            sdl2::event::Event::Quit { timestamp: _ } =>
                Some(UIEvent::Quit),

            sdl2::event::Event::KeyDown {
                timestamp: _,
                window_id: _,
                keycode: _,
                scancode: Some(scancode),
                keymod: _,
                repeat: false,
            } => {
                if let Some(ui_sc) = Self::sdl_sc_to_ui_sc(scancode) {
                    Some(UIEvent::Key { key: ui_sc, down: true })
                } else {
                    None
                }
            },

            sdl2::event::Event::KeyUp {
                timestamp: _,
                window_id: _,
                keycode: _,
                scancode: Some(scancode),
                keymod: _,
                repeat: _,
            } => {
                if let Some(ui_sc) = Self::sdl_sc_to_ui_sc(scancode) {
                    Some(UIEvent::Key { key: ui_sc, down: false })
                } else {
                    None
                }
            },

            sdl2::event::Event::Window {
                timestamp: _,
                window_id: _,
                win_event,
            } => {
                match win_event {
                    sdl2::event::WindowEvent::Resized(w, h) => {
                        self.update_rects(w as u32, h as u32);
                    },

                    sdl2::event::WindowEvent::Exposed => {
                        self.update_bg();
                    },

                    _ => (),
                }

                None
            },

            _ => {
                None
            },
        }
    }

    pub fn poll_event(&mut self) -> Option<UIEvent> {
        if let Some(evt) = self.sdl_evt_pump.poll_event() {
            if let Some(ui_event) = self.translate_event(evt) {
                Some(ui_event)
            } else {
                self.poll_event()
            }
        } else if let Some(sc) = &mut self.sc {
            sc.poll_event()
        } else {
            None
        }
    }

    pub fn wait_event(&mut self, timeout: std::time::Duration)
        -> Option<UIEvent>
    {
        let toms = timeout.as_millis() as u32;
        if let Some(sc) = &mut self.sc {
            if let Some(ui_event) = sc.wait_event(timeout) {
                return Some(ui_event);
            }
        }

        if let Some(evt) = self.sdl_evt_pump.wait_event_timeout(toms) {
            if let Some(ui_event) = self.translate_event(evt) {
                Some(ui_event)
            } else {
                self.wait_event(timeout)
            }
        } else {
            None
        }
    }

    pub fn enable_sgb_border(&mut self) {
        self.sgb_border = true;

        let (mut w, mut h) = self.wnd_cvs.output_size().unwrap();
        w = std::cmp::max(w, 256);
        h = std::cmp::max(h, 224);

        self.wnd_cvs.window_mut().set_size(w, h).unwrap();
        self.update_rects(w, h);
    }

    pub fn set_sgb_border(&mut self, pixels: &[u32; 256 * 224]) {
        let pixels8 = unsafe {
            std::slice::from_raw_parts(pixels.as_ptr() as *const u8,
                                       pixels.len() * 4)
        };

        self.sgb_border_txt.update(None, pixels8, 256 * 4).unwrap();
    }

    pub fn rumble(&mut self, state: bool) {
        if let Some(sc) = &mut self.sc {
            sc.rumble(state);
        }
    }

    pub fn set_fullscreen(&mut self, state: bool) {
        let fs_mode =
            if state {
                sdl2::video::FullscreenType::Desktop
            } else {
                sdl2::video::FullscreenType::Off
            };

        self.wnd_cvs.window_mut().set_fullscreen(fs_mode).unwrap();
    }

    pub fn set_paused(&mut self, paused: bool) {
        let dev = self.audio_dev.as_mut().unwrap();
        if paused {
            dev.pause();
        } else {
            dev.resume();
        }
    }
}


struct AudioOutput {
    buf: Arc<Mutex<Vec<f32>>>,
    buf_i: usize,
    buf_done: Sender<usize>,
}

impl sdl2::audio::AudioCallback for AudioOutput {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let inp_guard = self.buf.lock().unwrap();
        let inp = &*inp_guard;

        for i in 0..out.len() {
            out[i] = inp[self.buf_i + i];
        }
        self.buf_i = (self.buf_i + out.len()) % inp.len();

        self.buf_done.send(self.buf_i).unwrap();
    }
}
