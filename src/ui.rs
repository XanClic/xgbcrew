use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::system_state::{AudioOutputParams, UIEvent, UIScancode};


pub struct UI {
    sdl_audio: sdl2::AudioSubsystem,
    sdl_evt_pump: sdl2::EventPump,

    wnd_cvs: sdl2::render::Canvas<sdl2::video::Window>,
    lcd_txt: sdl2::render::Texture<'static>,

    audio_dev: Option<sdl2::audio::AudioDevice<AudioOutput>>
}

impl UI {
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

        Self {
            sdl_audio: audio,
            sdl_evt_pump: evt_pump,

            wnd_cvs: cvs,
            lcd_txt: lcd_txt,

            audio_dev: None,
        }
    }

    pub fn setup_audio(&mut self, params: AudioOutputParams) {
        let samples = {
            let buf_guard = params.buf.lock().unwrap();
            buf_guard.len()
        };

        let sound_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(params.freq as i32),
            channels: Some(params.channels as u8),
            samples: Some((samples / params.channels) as u16),
        };

        let adev_obj_gen = |_| {
            AudioOutput {
                buf: params.buf,
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
            std::mem::transmute::<&[u32], &[u8]>(pixels)
        };

        self.lcd_txt.update(None, pixels8, 160 * 4).unwrap();
        self.wnd_cvs.copy(&self.lcd_txt, None, None).unwrap();
        self.wnd_cvs.present();
    }

    fn sdl_sc_to_ui_sc(sdl_sc: sdl2::keyboard::Scancode) -> Option<UIScancode> {
        use sdl2::keyboard::Scancode;

        let ui_sc =
            match sdl_sc {
                Scancode::X         => UIScancode::X,
                Scancode::Z         => UIScancode::Z,

                Scancode::Space     => UIScancode::Space,
                Scancode::Return    => UIScancode::Return,
                Scancode::Backspace => UIScancode::Backspace,

                Scancode::Left      => UIScancode::Left,
                Scancode::Right     => UIScancode::Right,
                Scancode::Up        => UIScancode::Up,
                Scancode::Down      => UIScancode::Down,

                _ => { return None; },
            };

        Some(ui_sc)
    }

    pub fn poll_event(&mut self) -> Option<UIEvent> {
        if let Some(evt) = self.sdl_evt_pump.poll_event() {
            let ui_event =
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

                    _ => {
                        None
                    },
                };

            if ui_event.is_some() {
                ui_event
            } else {
                self.poll_event()
            }
        } else {
            None
        }
    }
}


struct AudioOutput {
    buf: Arc<Mutex<Vec<f32>>>,
    buf_done: Sender<()>,
}

impl sdl2::audio::AudioCallback for AudioOutput {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        let inp_guard = self.buf.lock().unwrap();
        let inp = &*inp_guard;

        for i in 0..out.len() {
            out[i] = inp[i];
        }

        self.buf_done.send(()).unwrap();
    }
}
