use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU8, Ordering};

use crate::io::{io_get_reg, io_set_addr, io_set_reg};
use crate::system_state::{IOReg, SystemState};

const FRAMES: usize = 256;


struct SharedState {
    lvol: f32,
    rvol: f32,
    channel_mask: u8,
}

impl SharedState {
    fn new() -> Self {
        Self {
            lvol: 7.0,
            rvol: 7.0,
            channel_mask: 0xf3,
        }
    }
}

struct ToneSweep {
    channel: usize,
    time: f32,
    enabled: bool,

    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,

    freq_x: u32,
    freq: f32,
    vol: f32,
    duty: f32,

    sample_count: usize,
    samples_limited: bool,

    env_enabled: bool,
    env_amplify: bool,
    env_len: f32,
    env_counter: f32,

    sweep_enabled: bool,
    sweep_up: bool,
    sweep_n: usize,
    sweep_time: f32,
    sweep_counter: f32,
}

impl ToneSweep {
    fn new(channel: usize) -> Self {
        let mut ts = Self {
            channel: channel,
            time: 0.0,
            enabled: false,

            nrx0: 0x80,
            nrx1: 0xbf,
            nrx2: 0xf3,
            nrx3: 0x00,
            nrx4: 0xbf,

            freq_x: 0,
            freq: 0.0,
            vol: 0.0,
            duty: 0.5,

            sample_count: 0,
            samples_limited: false,

            env_enabled: false,
            env_amplify: false,
            env_len: 0.0,
            env_counter: 0.0,

            sweep_enabled: false,
            sweep_up: false,
            sweep_n: 0,
            sweep_time: 0.0,
            sweep_counter: 0.0,
        };

        ts.update_freq(true);

        ts
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;

        if enabled {
            io_set_reg(IOReg::NR52,
                       io_get_reg(IOReg::NR52) | (1 << self.channel));
        } else {
            io_set_reg(IOReg::NR52,
                       io_get_reg(IOReg::NR52) & !(1 << self.channel));
        }
    }

    fn update_freq(&mut self, update_freq_x: bool) {
        if update_freq_x {
            self.freq_x = (self.nrx3 as u32) |
                          ((self.nrx4 as u32 & 0x07) << 8);
        }
        self.freq = 131072.0 / ((2048 - self.freq_x) as f32);
    }

    fn update_len(&mut self) {
        self.samples_limited = self.nrx4 & (1 << 6) != 0;
        if self.samples_limited {
            let x = self.nrx1 & 0x3f;
            self.sample_count = ((64 - x) as f32 * (44100.0 / 256.0)) as usize;
        }
    }

    fn update_duty(&mut self) {
        self.duty =
            match self.nrx1 >> 6 {
                0 => 0.125,
                1 => 0.25,
                2 => 0.5,
                3 => 0.75,
                _ => unreachable!(),
            };
    }

    fn update_envelope(&mut self) {
        self.vol = (self.nrx2 >> 4) as f32;
        if self.nrx2 & 0x07 != 0 {
            self.env_enabled = true;
            self.env_amplify = self.nrx2 & 0x08 != 0;
            self.env_len = (self.nrx2 & 0x07) as f32 * (44100.0 / 64.0);
            self.env_counter = 0.0;
        } else {
            self.env_enabled = false;
        }
    }

    fn update_sweep(&mut self) {
        let time_x = self.nrx0 >> 5;
        self.sweep_n = (self.nrx0 & 0x07) as usize;

        if self.sweep_n == 0 || time_x == 0 {
            self.sweep_enabled = false;
        } else {
            self.sweep_enabled = true;
            self.sweep_up = self.nrx0 & (1 << 3) == 0;
            self.sweep_time = time_x as f32 * (44100.0 / 128.0);
            self.sweep_counter = 0.0;
        }
    }

    fn initialize(&mut self) {
        self.update_envelope();
        self.update_len();
        self.update_duty();
        self.update_freq(true);
        self.update_sweep();

        self.set_enabled(true);
    }

    fn get_sample(&mut self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.samples_limited {
            if self.sample_count == 0 {
                self.set_enabled(false);
                return 0.0;
            }
            self.sample_count -= 1;
        }

        if self.env_enabled {
            self.env_counter += 1.0;
            if self.env_counter >= self.env_len {
                if self.env_amplify {
                    if self.vol > 14.5 {
                        self.env_enabled = false;
                    } else {
                        self.vol += 1.0;
                    }
                } else {
                    if self.vol < 0.5 {
                        self.env_enabled = false;
                    } else {
                        self.vol -= 1.0;
                    }
                }

                self.env_counter -= self.env_len;
            }
        }

        if self.sweep_enabled {
            self.sweep_counter += 1.0;
            if self.sweep_counter >= self.sweep_time {
                if self.sweep_up {
                    self.freq_x += self.freq_x >> self.sweep_n;
                    if self.freq_x >= 2048 {
                        self.set_enabled(false);
                        return 0.0;
                    }
                } else {
                    self.freq_x -= self.freq_x >> self.sweep_n;
                }

                let in_ofreq = (self.time * self.freq).fract();
                self.update_freq(false);
                self.time = in_ofreq / self.freq;

                self.sweep_counter -= self.sweep_time;
            }
        }

        let in_freq = (self.time * self.freq).fract();
        self.time = (self.time + 1.0 / 44100.0).fract();
        if in_freq >= self.duty { self.vol } else { -self.vol }
    }
}


struct Noise {
    channel: usize,
    enabled: bool,

    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,

    shift_time: f32,
    vol: f32,
    lfsr: u16,
    bits15: bool,
    output_counter: f32,

    sample_count: usize,
    samples_limited: bool,

    env_enabled: bool,
    env_amplify: bool,
    env_len: f32,
    env_counter: f32,
}

impl Noise {
    fn new(channel: usize) -> Self {
        Self {
            channel: channel,
            enabled: false,

            nrx1: 0xff,
            nrx2: 0x00,
            nrx3: 0x00,
            nrx4: 0xbf,

            shift_time: 0.0,
            vol: 0.0,
            lfsr: 0x7fff,
            bits15: false,
            output_counter: 0.0,

            sample_count: 0,
            samples_limited: false,

            env_enabled: false,
            env_amplify: false,
            env_len: 0.0,
            env_counter: 0.0,
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;

        if enabled {
            io_set_reg(IOReg::NR52,
                       io_get_reg(IOReg::NR52) | (1 << self.channel));
        } else {
            io_set_reg(IOReg::NR52,
                       io_get_reg(IOReg::NR52) & !(1 << self.channel));
        }
    }

    fn update_freq(&mut self) {
        let mut r = self.nrx3 & 0x07;
        let s = self.nrx3 >> 4;

        if r == 0 {
            r = 1;
        } else {
            r *= 2;
        }

        self.shift_time = (r as f32 * ((s + 1) as f32).exp2()) / 1048576.0;
    }

    fn update_len(&mut self) {
        self.samples_limited = self.nrx4 & (1 << 6) != 0;
        if self.samples_limited {
            let x = self.nrx1 & 0x3f;
            self.sample_count = ((64 - x) as f32 * (44100.0 / 256.0)) as usize;
        }
    }

    fn update_envelope(&mut self) {
        self.vol = (self.nrx2 >> 4) as f32;
        if self.nrx2 & 0x07 != 0 {
            self.env_enabled = true;
            self.env_amplify = self.nrx2 & 0x08 != 0;
            self.env_len = (self.nrx2 & 0x07) as f32 * (44100.0 / 64.0);
            self.env_counter = 0.0;
        } else {
            self.env_enabled = false;
        }
    }

    fn initialize(&mut self) {
        self.update_envelope();
        self.update_len();
        self.update_freq();

        self.bits15 = self.nrx3 & (1 << 3) == 0;
        if self.bits15 {
            self.lfsr = 0x7fff;
        } else {
            self.lfsr = 0x7f;
        }
        self.output_counter = 0.0;

        self.set_enabled(true);
    }

    fn shift(&mut self) {
        if self.bits15 {
            self.lfsr =
                (self.lfsr >> 1) |
                (((self.lfsr & (1 << 1)) << 13) ^
                 ((self.lfsr & (1 << 0)) << 14));
        } else {
            self.lfsr =
                (self.lfsr >> 1) |
                (((self.lfsr & (1 << 1)) << 5) ^
                 ((self.lfsr & (1 << 0)) << 6));
        }
    }

    fn get_sample(&mut self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.samples_limited {
            if self.sample_count == 0 {
                self.set_enabled(false);
                return 0.0;
            }
            self.sample_count -= 1;
        }

        if self.env_enabled {
            self.env_counter += 1.0;
            if self.env_counter >= self.env_len {
                if self.env_amplify {
                    if self.vol > 14.5 {
                        self.env_enabled = false;
                    } else {
                        self.vol += 1.0;
                    }
                } else {
                    if self.vol < 0.5 {
                        self.env_enabled = false;
                    } else {
                        self.vol -= 1.0;
                    }
                }

                self.env_counter -= self.env_len;
            }
        }

        self.output_counter += 1.0 / 44100.0;
        while self.output_counter >= self.shift_time {
            self.shift();
            self.output_counter -= self.shift_time;
        }

        if self.lfsr & 1 == 0 { -self.vol } else { self.vol }
    }
}


pub struct SoundState {
    sdl_audio: sdl2::AudioSubsystem,
    adev: Option<sdl2::audio::AudioDevice<Output>>,

    outbuf: Arc<Mutex<Vec<i8>>>,
    outbuf_done: Arc<AtomicU8>,

    obuf_i: usize,
    obuf_i_cycles: f32,
    shared: SharedState,

    ch1: ToneSweep,
    ch2: ToneSweep,
    ch4: Noise,
}

impl SoundState {
    pub fn new(sdl: &sdl2::Sdl) -> Self {
        let mut outbuf = Vec::<i8>::new();
        outbuf.resize(FRAMES * 2, 0i8);

        Self {
            sdl_audio: sdl.audio().unwrap(),
            adev: None,

            outbuf: Arc::new(Mutex::new(outbuf)),
            outbuf_done: Arc::new(AtomicU8::new(1)),

            obuf_i: 0,
            obuf_i_cycles: 0.0,
            shared: SharedState::new(),

            ch1: ToneSweep::new(0),
            ch2: ToneSweep::new(1),
            ch4: Noise::new(3),
        }
    }

    fn reset_regs(&mut self) {
        io_set_reg(IOReg::NR10, 0x80);
        io_set_reg(IOReg::NR11, 0xbf);
        io_set_reg(IOReg::NR12, 0xf3);
        io_set_reg(IOReg::NR14, 0xbf);
        io_set_reg(IOReg::NR21, 0x3f);
        io_set_reg(IOReg::NR22, 0x00);
        io_set_reg(IOReg::NR24, 0xbf);
        io_set_reg(IOReg::NR30, 0x7f);
        io_set_reg(IOReg::NR31, 0xff);
        io_set_reg(IOReg::NR32, 0x9f);
        io_set_reg(IOReg::NR33, 0xbf);
        io_set_reg(IOReg::NR41, 0xff);
        io_set_reg(IOReg::NR42, 0x00);
        io_set_reg(IOReg::NR43, 0x00);
        io_set_reg(IOReg::NR44, 0xbf);
        io_set_reg(IOReg::NR50, 0x77);
        io_set_reg(IOReg::NR51, 0xf3);
        io_set_reg(IOReg::NR52, 0xf1);

        self.obuf_i = 0;
        self.obuf_i_cycles = 0.0;
        self.shared = SharedState::new();
        self.ch1 = ToneSweep::new(0);
        self.ch2 = ToneSweep::new(1);
        self.ch4 = Noise::new(3);

        self.outbuf_done.store(0, Ordering::Release);
    }

    pub fn init_system_state(sys_state: &mut SystemState) {
        SoundState::reset_regs(&mut sys_state.sound);

        let sound_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(44100),
            channels: Some(2),
            samples: Some(FRAMES as u16),
        };

        let obuf = sys_state.sound.outbuf.clone();
        let obuf_done = sys_state.sound.outbuf_done.clone();

        sys_state.sound.adev = Some(
            sys_state.sound.sdl_audio
                .open_playback(None, &sound_spec,
                               |_| {
                                   Output {
                                       outbuf: obuf,
                                       outbuf_done: obuf_done,
                                   }
                               }).unwrap()
        );

        sys_state.sound.adev.as_mut().unwrap().resume();
    }

    /* @cycles must be in double-speed cycles */
    pub fn add_cycles(&mut self, cycles: u32) {
        self.obuf_i_cycles += cycles as f32;

        while self.obuf_i_cycles >= (2097152.0 / 44100.0) {
            if self.obuf_i == FRAMES * 2 {
                while self.outbuf_done.load(Ordering::Acquire) == 0 {
                }

                self.outbuf_done.store(0, Ordering::Release);
                self.obuf_i = 0;
            }

            let mut out_guard = self.outbuf.lock().unwrap();
            let out = &mut *out_guard;

            let ch1 = self.ch1.get_sample();
            let ch2 = self.ch2.get_sample();
            let ch4 = self.ch4.get_sample();

            let cm = self.shared.channel_mask;
            let ch1_f = (if cm & (1 << 0) != 0 { ch1 } else { 0.0 },
                         if cm & (1 << 4) != 0 { ch1 } else { 0.0 });
            let ch2_f = (if cm & (1 << 1) != 0 { ch2 } else { 0.0 },
                         if cm & (1 << 5) != 0 { ch2 } else { 0.0 });
            let ch4_f = (if cm & (1 << 3) != 0 { ch4 } else { 0.0 },
                         if cm & (1 << 7) != 0 { ch4 } else { 0.0 });

            let mut cht_f = (
                    (ch1_f.0 + ch2_f.0 + ch4_f.0) *
                        self.shared.lvol * (127.0 / (15.0 * 7.0)),
                    (ch1_f.1 + ch2_f.1 + ch4_f.0) *
                        self.shared.rvol * (127.0 / (15.0 * 7.0))
                );

            if cht_f.0 > 127.0 {
                cht_f.0 = 127.0;
            } else if cht_f.0 < -127.0 {
                cht_f.0 = -127.0;
            }

            if cht_f.1 > 127.0 {
                cht_f.1 = 127.0;
            } else if cht_f.1 < -127.0 {
                cht_f.1 = -127.0;
            }

            out[self.obuf_i + 0] = cht_f.0 as i8;
            out[self.obuf_i + 1] = cht_f.1 as i8;

            self.obuf_i_cycles -= 2097152.0 / 44100.0;
            self.obuf_i += 2;
        }
    }
}


struct Output {
    outbuf: Arc<Mutex<Vec<i8>>>,
    outbuf_done: Arc<AtomicU8>,
}

impl sdl2::audio::AudioCallback for Output {
    type Channel = i8;

    fn callback(&mut self, out: &mut [i8]) {
        let inp_guard = self.outbuf.lock().unwrap();
        let inp = &*inp_guard;

        for i in 0..out.len() {
            out[i] = inp[i];
        }

        self.outbuf_done.store(1, Ordering::Release);
    }
}


pub fn sound_write(sys_state: &mut SystemState, addr: u16, mut val: u8)
{
    let s = &mut sys_state.sound;
    let nr52 = io_get_reg(IOReg::NR52);

    if nr52 & 0x80 == 0 && addr != 0x26 {
        return;
    }

    match addr {
        0x10 => {
            s.ch1.nrx0 = val;
            val &= 0x7f;
        },

        0x11 => {
            s.ch1.nrx1 = val;
            val &= 0xc0;
        },

        0x12 => {
            s.ch1.nrx2 = val;
        },

        0x13 => {
            s.ch1.nrx3 = val;
            val = 0;
        },

        0x14 => {
            s.ch1.nrx4 = val;

            if val & 0x80 != 0 {
                s.ch1.initialize();
            }

            val &= 0x40;
        },

        0x15 => {
            val = 0;
        },

        0x16 => {
            s.ch2.nrx1 = val;
            val &= 0xc0;
        },

        0x17 => {
            s.ch2.nrx2 = val;
        },

        0x18 => {
            s.ch2.nrx3 = val;
            val = 0;
        },

        0x19 => {
            s.ch2.nrx4 = val;

            if val & 0x80 != 0 {
                s.ch2.initialize();
            }

            val &= 0x40;
        },

        0x20 => {
            s.ch4.nrx1 = val;
            val = 0;
        },

        0x21 => {
            s.ch4.nrx2 = val;
        },

        0x22 => {
            s.ch4.nrx3 = val;
        },

        0x23 => {
            s.ch4.nrx4 = val;

            if val & 0x80 != 0 {
                s.ch4.initialize();
            }

            val &= 0x40;
        },

        0x24 => {
            s.shared.lvol = ((val >> 0) & 0x07) as f32;
            s.shared.rvol = ((val >> 4) & 0x07) as f32;
        },

        0x25 => {
            s.shared.channel_mask = val;
        },

        0x26 => {
            val = (val & 0x80) | (nr52 & 0xf);
            if val & 0x80 == 0 {
                SoundState::reset_regs(s);
            }
        },

        _ => (),
    }

    io_set_addr(addr, val);
}
