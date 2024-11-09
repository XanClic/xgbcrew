use std::cmp;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

use crate::address_space::AddressSpace;
use crate::io::IOSpace;
use crate::system_state::{IOReg, SystemState};
use crate::ui::AudioOutputParams;

/*
 * We do real-time synchronization through audio, so we need at least
 * one sync point per frame (768 ~= 44100 / 60)
 */

/* Number of frames to feed the audio driver per sync point */
const FRAMES: usize = 768;

/* Number of samples to feed the audio driver per sync point */
const SAMPLES: usize = FRAMES * 2;

/*
 * Number of buffers to use
 * 1: Generate a single buffer, wait for the audio driver to use it,
 *    then continue.
 * 2: Generate two buffers, wait for the audio driver to use the first
 *    one, then reclaim its space.  That means double the delay, but
 *    there is always one buffer in reserve.
 * (Higher numbers accordingly.)
 */
const BUFCOUNT: usize = 2;

/* Total sample count of all sound buffers */
const BUFSZ: usize = SAMPLES * BUFCOUNT;


#[derive(SaveState)]
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

#[derive(SaveState)]
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
            channel,
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

    fn set_enabled(&mut self, addr_space: &mut AddressSpace, enabled: bool) {
        self.enabled = enabled;

        let nr52 = addr_space.io_get_reg(IOReg::NR52);
        if enabled {
            addr_space.io_set_reg(IOReg::NR52, nr52 | (1 << self.channel));
        } else {
            addr_space.io_set_reg(IOReg::NR52, nr52 & !(1 << self.channel));
        }
    }

    fn update_freq(&mut self, update_freq_x: bool) {
        let in_ofreq = (self.time * self.freq).fract();

        if update_freq_x {
            self.freq_x = (self.nrx3 as u32) |
                          ((self.nrx4 as u32 & 0x07) << 8);
        }
        self.freq = 131072.0 / ((2048 - self.freq_x) as f32);

        self.time = in_ofreq / self.freq;
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
        let time_x = (self.nrx0 & 0x70) >> 4;
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

    fn initialize(&mut self, addr_space: &mut AddressSpace) {
        self.update_envelope();
        self.update_len();
        self.update_duty();
        self.update_freq(true);
        self.update_sweep();

        self.set_enabled(addr_space, true);
    }

    fn get_sample(&mut self, addr_space: &mut AddressSpace) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.samples_limited {
            if self.sample_count == 0 {
                self.set_enabled(addr_space, false);
                return 0.0;
            }
            self.sample_count -= 1;
        }

        if self.env_enabled {
            self.env_counter += 1.0;
            if self.env_counter >= self.env_len {
                match self.env_amplify {
                    true => {
                        if self.vol <= 14.5 {
                            self.vol += 1.0;
                        } else {
                            self.env_enabled = false;
                        }
                    }

                    false => {
                        if self.vol >= 0.5 {
                            self.vol -= 1.0;
                        } else {
                            self.env_enabled = false;
                        }
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
                        self.set_enabled(addr_space, false);
                        return 0.0;
                    }
                } else {
                    self.freq_x -= self.freq_x >> self.sweep_n;
                }
                self.update_freq(false);
                self.sweep_counter -= self.sweep_time;
            }
        }

        self.time += 1.0 / 44100.0;
        let in_freq = (self.time * self.freq).fract();
        self.time = in_freq / self.freq;

        if in_freq >= self.duty { self.vol } else { 0.0 }
    }
}


#[derive(SaveState)]
struct Wave {
    channel: usize,
    enabled: bool,
    soft_stopped: bool,

    nrx0: u8,
    nrx1: u8,
    nrx2: u8,
    nrx3: u8,
    nrx4: u8,

    samples: [u8; 16],

    next_vol: f32,
    vol: f32,
    next_sample_time: f32,
    sample_time: f32,
    sample_counter: f32,
    sample_i: usize,

    out_sample_count: usize,
    out_samples_limited: bool,
}

impl Wave {
    fn new(channel: usize) -> Self {
        Self {
            channel,
            enabled: false,
            soft_stopped: false,

            nrx0: 0x7f,
            nrx1: 0xff,
            nrx2: 0x9f,
            nrx3: 0xbf,
            nrx4: 0x00,

            samples: [0u8; 16],

            next_vol: 0.0,
            vol: 0.0,
            next_sample_time: 0.0,
            sample_time: 0.0,
            sample_counter: 0.0,
            sample_i: 0,

            out_sample_count: 0,
            out_samples_limited: false,
        }
    }

    fn set_enabled(&mut self, addr_space: &mut AddressSpace, enabled: bool) {
        self.enabled = enabled;

        let nr52 = addr_space.io_get_reg(IOReg::NR52);
        if enabled {
            addr_space.io_set_reg(IOReg::NR52, nr52 | (1 << self.channel));
        } else {
            addr_space.io_set_reg(IOReg::NR52, nr52 & !(1 << self.channel));
        }
    }

    fn update_freq(&mut self) {
        let freq_x = (self.nrx3 as u32) |
                     ((self.nrx4 as u32 & 0x07) << 8);
        self.next_sample_time = ((2048 - freq_x) as f32) / 65536.0 / 32.0;
    }

    fn update_len(&mut self) {
        self.out_samples_limited = self.nrx4 & (1 << 6) != 0;
        if self.out_samples_limited {
            self.out_sample_count = ((256 - self.nrx1 as u32) as f32 *
                                    (44100.0 / 256.0)) as usize;
        }
    }

    fn update_vol(&mut self) {
        self.next_vol =
            match (self.nrx2 >> 5) & 0x03 {
                0 =>  0.0,
                1 => -1.0,
                2 => -0.5,
                3 => -0.25,
                _ => unreachable!(),
            };
    }

    fn pull_regs(&mut self, addr_space: &mut AddressSpace) {
        if self.sample_i == 0 {
            for i in 0..16 {
                self.samples[i] = addr_space.io_get_addr(0x30 + i as u16);
            }
            self.vol = self.next_vol;
            self.sample_time = self.next_sample_time;
        }
    }

    fn initialize(&mut self, addr_space: &mut AddressSpace) {
        self.update_len();
        self.update_freq();
        self.update_vol();

        self.set_enabled(addr_space, true);
    }

    fn get_sample(&mut self, addr_space: &mut AddressSpace) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.soft_stopped && self.sample_i == 0 {
            self.set_enabled(addr_space, false);
            return 0.0;
        }

        if self.out_samples_limited {
            if self.out_sample_count == 0 {
                if self.sample_i == 0 {
                    self.set_enabled(addr_space, false);
                    return 0.0;
                }
            } else {
                self.out_sample_count -= 1;
            }
        }

        let mut in_sample_t = (self.samples[self.sample_i / 2] as usize
                               >> (4 - (self.sample_i % 2) * 4))
                              & 0x0f;
        let mut in_sample_count = 1.0;

        self.sample_counter += 1.0 / 44100.0;
        while self.sample_counter >= self.sample_time {
            self.sample_i = (self.sample_i + 1) % 32;
            self.sample_counter -= self.sample_time;

            self.pull_regs(addr_space);

            if self.sample_counter >= self.sample_time {
                in_sample_t += (self.samples[self.sample_i / 2] as usize
                                >> (4 - (self.sample_i % 2) * 4))
                               & 0x0f;
                in_sample_count += 1.0;
            }
        }

        (in_sample_t as f32 / in_sample_count) * self.vol
    }
}


#[derive(SaveState)]
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

    #[savestate(skip)]
    last_raw_sample: f32,
    #[savestate(skip)]
    position: f32,
    #[savestate(skip)]
    velocity: f32,
}

impl Noise {
    fn new(channel: usize) -> Self {
        Self {
            channel,
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

            last_raw_sample: 0.0,
            position: 0.0,
            velocity: 0.0,
        }
    }

    fn set_enabled(&mut self, addr_space: &mut AddressSpace, enabled: bool) {
        self.enabled = enabled;

        let nr52 = addr_space.io_get_reg(IOReg::NR52);
        if enabled {
            addr_space.io_set_reg(IOReg::NR52, nr52 | (1 << self.channel));
        } else {
            addr_space.io_set_reg(IOReg::NR52, nr52 & !(1 << self.channel));
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

    fn initialize(&mut self, addr_space: &mut AddressSpace) {
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

        self.set_enabled(addr_space, true);
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

    fn get_raw_sample(&mut self, addr_space: &mut AddressSpace) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.samples_limited {
            if self.sample_count == 0 {
                self.set_enabled(addr_space, false);
                return 0.0;
            }
            self.sample_count -= 1;
        }

        if self.env_enabled {
            self.env_counter += 1.0;
            if self.env_counter >= self.env_len {
                match self.env_amplify {
                    true => {
                        if self.vol <= 14.5 {
                            self.vol += 1.0;
                        } else {
                            self.env_enabled = false;
                        }
                    }

                    false => {
                        if self.vol >= 0.5 {
                            self.vol -= 1.0;
                        } else {
                            self.env_enabled = false;
                        }
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

        if self.lfsr & 1 != 0 { self.vol } else { 0.0 }
    }

    fn get_sample(&mut self, addr_space: &mut AddressSpace) -> f32 {
        let raw_sample = self.get_raw_sample(addr_space);
        let diff = raw_sample - self.last_raw_sample;

        self.last_raw_sample = raw_sample;
        self.velocity = self.velocity * 0.5 + diff * 0.5;
        self.position += self.velocity;

        self.position
    }
}


#[derive(SaveState)]
pub struct SoundState {
    #[savestate(skip)]
    outbuf: Arc<Mutex<Vec<f32>>>,
    #[savestate(skip)]
    intbuf: Vec<f32>,
    #[savestate(skip)]
    outbuf_done: Receiver<usize>,
    #[savestate(skip)]
    outbuf_done_handout: Option<Sender<usize>>,

    ibuf_i: usize,
    #[savestate(skip)]
    obuf_i: usize,
    ibuf_i_cycles: f32,
    shared: SharedState,

    ch1: ToneSweep,
    ch2: ToneSweep,
    ch3: Wave,
    ch4: Noise,

    ch3_l: bool,
    ch3_r: bool,

    #[savestate(skip)]
    postprocess: bool,

    #[savestate(skip)]
    last_raw_sample: (f32, f32),
    #[savestate(skip)]
    velocity: (f32, f32),
    #[savestate(skip)]
    position: (f32, f32),
}

impl SoundState {
    pub fn new() -> Self {
        let mut outbuf = Vec::<f32>::new();
        outbuf.resize(BUFSZ, 0.0);

        let mut intbuf = Vec::<f32>::new();
        intbuf.resize(BUFSZ, 0.0);

        let (snd, rcv) = channel();

        Self {
            outbuf: Arc::new(Mutex::new(outbuf)),
            intbuf,
            outbuf_done: rcv,
            outbuf_done_handout: Some(snd),

            ibuf_i: 0,
            obuf_i: 0,
            ibuf_i_cycles: 0.0,
            shared: SharedState::new(),

            ch1: ToneSweep::new(0),
            ch2: ToneSweep::new(1),
            ch3: Wave::new(2),
            ch4: Noise::new(3),

            ch3_l: false,
            ch3_r: false,

            postprocess: false,

            last_raw_sample: (0.0, 0.0),
            velocity: (0.0, 0.0),
            position: (0.0, 0.0),
        }
    }

    fn reset_regs(&mut self, addr_space: &mut AddressSpace) {
        addr_space.io_set_reg(IOReg::NR10, 0x80);
        addr_space.io_set_reg(IOReg::NR11, 0xbf);
        addr_space.io_set_reg(IOReg::NR12, 0xf3);
        addr_space.io_set_reg(IOReg::NR14, 0xbf);
        addr_space.io_set_reg(IOReg::NR21, 0x3f);
        addr_space.io_set_reg(IOReg::NR22, 0x00);
        addr_space.io_set_reg(IOReg::NR24, 0xbf);
        addr_space.io_set_reg(IOReg::NR30, 0x7f);
        addr_space.io_set_reg(IOReg::NR31, 0xff);
        addr_space.io_set_reg(IOReg::NR32, 0x9f);
        addr_space.io_set_reg(IOReg::NR33, 0xbf);
        addr_space.io_set_reg(IOReg::NR41, 0xff);
        addr_space.io_set_reg(IOReg::NR42, 0x00);
        addr_space.io_set_reg(IOReg::NR43, 0x00);
        addr_space.io_set_reg(IOReg::NR44, 0xbf);
        addr_space.io_set_reg(IOReg::NR50, 0x77);
        addr_space.io_set_reg(IOReg::NR51, 0xf3);
        addr_space.io_set_reg(IOReg::NR52, 0xf1);

        self.ibuf_i = 0;
        self.ibuf_i_cycles = 0.0;
        self.shared = SharedState::new();
        self.ch1 = ToneSweep::new(0);
        self.ch2 = ToneSweep::new(1);
        self.ch3 = Wave::new(2);
        self.ch4 = Noise::new(3);
    }

    pub fn get_audio_params(&mut self) -> AudioOutputParams {
        AudioOutputParams {
            freq: 44100,
            channels: 2,

            buf: self.outbuf.clone(),
            buf_step: SAMPLES,
            buf_done: self.outbuf_done_handout.take().unwrap(),
        }
    }

    fn gen_one_frame(&mut self, addr_space: &mut AddressSpace) -> (f32, f32) {
        let ch1 = self.ch1.get_sample(addr_space);
        let ch2 = self.ch2.get_sample(addr_space);
        let ch3 = self.ch3.get_sample(addr_space);
        let ch4 = self.ch4.get_sample(addr_space);

        let cm = self.shared.channel_mask;
        let ch1_f = (if cm & (1 << 4) != 0 { ch1 } else { 0.0 },
                     if cm & (1 << 0) != 0 { ch1 } else { 0.0 });
        let ch2_f = (if cm & (1 << 5) != 0 { ch2 } else { 0.0 },
                     if cm & (1 << 1) != 0 { ch2 } else { 0.0 });
        let ch3_f = (if self.ch3_l { ch3 } else { 0.0 },
                     if self.ch3_r { ch3 } else { 0.0 });
        let ch4_f = (if cm & (1 << 7) != 0 { ch4 } else { 0.0 },
                     if cm & (1 << 3) != 0 { ch4 } else { 0.0 });

        if self.ch3.sample_i == 0 {
            self.ch3_l = cm & (1 << 6) != 0;
            self.ch3_r = cm & (1 << 2) != 0;
        }

        let cht_f = (
                (ch1_f.0 + ch2_f.0 + ch3_f.0 + ch4_f.0) *
                    self.shared.lvol * 0.005,
                (ch1_f.1 + ch2_f.1 + ch3_f.1 + ch4_f.1) *
                    self.shared.rvol * 0.005
            );

        let diff = (cht_f.0 - self.last_raw_sample.0,
                    cht_f.1 - self.last_raw_sample.1);

        let force = (diff.0 - self.position.0 * 0.04,
                     diff.1 - self.position.1 * 0.04);

        self.last_raw_sample.0 = cht_f.0;
        self.last_raw_sample.1 = cht_f.1;

        self.velocity.0 = self.velocity.0 * 0.1 + force.0 * 0.9;
        self.velocity.1 = self.velocity.1 * 0.1 + force.1 * 0.9;

        self.position.0 += self.velocity.0;
        self.position.1 += self.velocity.1;

        (self.position.0, self.position.1)
    }

    /* @cycles must be in double-speed cycles */
    pub fn add_cycles(&mut self, addr_space: &mut AddressSpace,
                      cycles: u32, realtime: bool)
    {
        self.ibuf_i_cycles += cycles as f32;

        while self.ibuf_i_cycles >= (2097152.0 / 44100.0) {
            let (l, r) = self.gen_one_frame(addr_space);

            self.intbuf[self.ibuf_i] = l;
            self.intbuf[self.ibuf_i + 1] = r;

            self.ibuf_i_cycles -= 2097152.0 / 44100.0;
            self.ibuf_i = (self.ibuf_i + 2) % BUFSZ;

            if self.ibuf_i % SAMPLES == 0 {
                let start = (self.ibuf_i + BUFSZ - SAMPLES) % BUFSZ;
                let end = if self.ibuf_i == 0 { BUFSZ } else { self.ibuf_i };

                {
                    let mut out_guard = self.outbuf.lock().unwrap();
                    let out = &mut *out_guard;

                    out[start..end].copy_from_slice(&self.intbuf[start..end]);
                }

                if self.ibuf_i == self.obuf_i {
                    self.obuf_i =
                        if realtime {
                            self.outbuf_done.recv().unwrap()
                        } else {
                            self.outbuf_done.try_recv().unwrap_or(0)
                        };
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn fill_outbuf(&mut self, addr_space: &mut AddressSpace, output: &mut [f32]) {
        for i in 0..(output.len() / 2) {
            let (l, r) = self.gen_one_frame(addr_space);

            output[i * 2] = l;
            output[i * 2 + 1] = r;
        }
    }

    pub fn set_postprocessing(&mut self, postprocess: bool) {
        self.postprocess = postprocess;
    }
}


pub fn sound_write(sys_state: &mut SystemState, addr: u16, mut val: u8)
{
    let s = &mut sys_state.sound;
    let addr_space = &mut sys_state.addr_space;
    let nr52 = addr_space.io_get_reg(IOReg::NR52);

    if nr52 & 0x80 == 0 && addr != 0x26 {
        return;
    }

    match addr {
        0x10 => {
            s.ch1.nrx0 = val;
            s.ch1.update_sweep();
            val &= 0x7f;
        },

        0x11 => {
            s.ch1.nrx1 = val;
            s.ch1.update_duty();
            s.ch1.update_len();
            val &= 0xc0;
        },

        0x12 => {
            s.ch1.nrx2 = val;
            s.ch1.update_envelope();
        },

        0x13 => {
            s.ch1.nrx3 = val;
            s.ch1.update_freq(true);
            val = 0;
        },

        0x14 => {
            s.ch1.nrx4 = val;
            s.ch1.update_freq(true);

            s.ch1.samples_limited = val & (1 << 6) != 0;

            if val & 0x80 != 0 {
                s.ch1.initialize(addr_space);
            }

            val &= 0x40;
        },

        0x15 => {
            val = 0;
        },

        0x16 => {
            s.ch2.nrx1 = val;
            s.ch2.update_duty();
            s.ch2.update_len();
            val &= 0xc0;
        },

        0x17 => {
            s.ch2.nrx2 = val;
            s.ch2.update_envelope();
        },

        0x18 => {
            s.ch2.nrx3 = val;
            s.ch2.update_freq(true);
            val = 0;
        },

        0x19 => {
            s.ch2.nrx4 = val;
            s.ch2.update_freq(true);

            s.ch2.samples_limited = val & (1 << 6) != 0;

            if val & 0x80 != 0 {
                s.ch2.initialize(addr_space);
            }

            val &= 0x40;
        },

        0x1a => {
            s.ch3.nrx0 = val;

            if val & (1 << 7) == 0 {
                if s.ch3.enabled {
                    s.ch3.soft_stopped = true;
                }
            } else if s.ch3.soft_stopped {
                s.ch3.soft_stopped = false;
                s.ch3.set_enabled(addr_space, true);
            }

            val &= 0x80;
        },

        0x1b => {
            s.ch3.nrx1 = val;
            s.ch3.update_len();
        },

        0x1c => {
            s.ch3.nrx2 = val;
            s.ch3.update_vol();
            val &= 0x60;
        },

        0x1d => {
            s.ch3.nrx3 = val;
            s.ch3.update_freq();
            val = 0;
        },

        0x1e => {
            s.ch3.nrx4 = val;
            s.ch3.update_freq();

            s.ch3.out_samples_limited = val & (1 << 6) != 0;

            if val & 0x80 != 0 {
                s.ch3.initialize(addr_space);
            }

            val &= 0x40;
        },

        0x1f => {
            val = 0;
        },

        0x20 => {
            s.ch4.nrx1 = val;
            s.ch4.update_len();
            val = 0;
        },

        0x21 => {
            s.ch4.nrx2 = val;
            s.ch4.update_envelope();
        },

        0x22 => {
            s.ch4.nrx3 = val;
            s.ch4.update_freq();
        },

        0x23 => {
            s.ch4.nrx4 = val;

            s.ch4.samples_limited = val & (1 << 6) != 0;

            if val & 0x80 != 0 {
                s.ch4.initialize(addr_space);
            }

            val &= 0x40;
        },

        0x24 => {
            // Never mute
            s.shared.lvol = (cmp::max((val >> 4) & 0x07, 1)) as f32;
            s.shared.rvol = (cmp::max(val & 0x07, 1)) as f32;
        },

        0x25 => {
            s.shared.channel_mask = val;
        },

        0x26 => {
            val = (val & 0x80) | (nr52 & 0xf);
            if val & 0x80 == 0 {
                s.reset_regs(addr_space);
            }
        },

        0x30..=0x3f => (),

        _ => unreachable!(),
    }

    addr_space.io_set_addr(addr, val);
}
