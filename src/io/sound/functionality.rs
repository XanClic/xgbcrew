use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

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


pub struct AudioOutputParams {
    pub freq: usize,
    pub channels: usize,

    pub buf: Arc<Mutex<Vec<f32>>>,
    pub buf_step: usize,
    pub buf_done: Sender<usize>,
}

pub trait GlobalAudioState {
    fn enable_channel(&mut self, channel: usize, enabled: bool);
    fn wave_sample(&self, i: usize) -> u8;
}


#[derive(SaveState)]
pub struct SharedState {
    pub lvol: f32,
    pub rvol: f32,
    pub channel_mask: u8,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            lvol: 7.0,
            rvol: 7.0,
            channel_mask: 0xf3,
        }
    }
}

#[derive(SaveState)]
pub struct ToneSweep {
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
    pub fn new(channel: usize) -> Self {
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

    pub fn set_nrx0(&mut self, val: u8) {
        self.nrx0 = val;
        self.update_sweep();
    }

    pub fn set_nrx1(&mut self, val: u8) {
        self.nrx1 = val;
        self.update_duty();
        self.update_len();
    }

    pub fn set_nrx2(&mut self, val: u8) {
        self.nrx2 = val;
        self.update_envelope();
    }

    pub fn set_nrx3(&mut self, val: u8) {
        self.nrx3 = val;
        self.update_freq(true);
    }

    pub fn set_nrx4<S: GlobalAudioState>(&mut self, val: u8, gas: &mut S) {
        self.nrx4 = val;
        self.update_freq(true);

        self.samples_limited = val & (1 << 6) != 0;

        if val & 0x80 != 0 {
            self.initialize(gas);
        }
    }

    /// `enable_ext` marks this channel as enabled/disabled in NR52.  It takes the channel index
    /// and whether it should be enabled or not.
    pub fn set_enabled<S: GlobalAudioState>(&mut self, gas: &mut S, enabled: bool) {
        self.enabled = enabled;
        gas.enable_channel(self.channel, enabled);
    }

    pub fn update_freq(&mut self, update_freq_x: bool) {
        let in_ofreq = (self.time * self.freq).fract();

        if update_freq_x {
            self.freq_x = (self.nrx3 as u32) |
                          ((self.nrx4 as u32 & 0x07) << 8);
        }
        self.freq = 131072.0 / ((2048 - self.freq_x) as f32);

        self.time = in_ofreq / self.freq;
    }

    pub fn update_len(&mut self) {
        self.samples_limited = self.nrx4 & (1 << 6) != 0;
        if self.samples_limited {
            let x = self.nrx1 & 0x3f;
            self.sample_count = ((64 - x) as f32 * (44100.0 / 256.0)) as usize;
        }
    }

    pub fn update_duty(&mut self) {
        self.duty =
            match self.nrx1 >> 6 {
                0 => 0.125,
                1 => 0.25,
                2 => 0.5,
                3 => 0.75,
                _ => unreachable!(),
            };
    }

    pub fn update_envelope(&mut self) {
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

    pub fn update_sweep(&mut self) {
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

    pub fn initialize<S: GlobalAudioState>(&mut self, gas: &mut S) {
        self.update_envelope();
        self.update_len();
        self.update_duty();
        self.update_freq(true);
        self.update_sweep();

        self.set_enabled(gas, true);
    }

    pub fn get_sample<S: GlobalAudioState>(&mut self, gas: &mut S) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.samples_limited {
            if self.sample_count == 0 {
                self.set_enabled(gas, false);
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
                        self.set_enabled(gas, false);
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
pub struct Wave {
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
    pub fn new(channel: usize) -> Self {
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

    pub fn set_nrx0<S: GlobalAudioState>(&mut self, val: u8, gas: &mut S) {
        self.nrx0 = val;

        if val & (1 << 7) == 0 {
            if self.enabled {
                self.soft_stopped = true;
            }
        } else if self.soft_stopped {
            self.soft_stopped = false;
            self.set_enabled(gas, true);
        }
    }

    pub fn set_nrx1(&mut self, val: u8) {
        self.nrx1 = val;
        self.update_len();
    }

    pub fn set_nrx2(&mut self, val: u8) {
        self.nrx2 = val;
        self.update_vol();
    }

    pub fn set_nrx3(&mut self, val: u8) {
        self.nrx3 = val;
        self.update_freq();
    }

    pub fn set_nrx4<S: GlobalAudioState>(&mut self, val: u8, gas: &mut S) {
        self.nrx4 = val;
        self.update_freq();

        self.out_samples_limited = val & (1 << 6) != 0;

        if val & 0x80 != 0 {
            self.initialize(gas);
        }
    }

    /// `enable_ext` marks this channel as enabled/disabled in NR52.  It takes the channel index
    /// and whether it should be enabled or not.
    pub fn set_enabled<S: GlobalAudioState>(&mut self, gas: &mut S, enabled: bool) {
        self.enabled = enabled;
        gas.enable_channel(self.channel, enabled);
    }

    pub fn update_freq(&mut self) {
        let freq_x = (self.nrx3 as u32) |
                     ((self.nrx4 as u32 & 0x07) << 8);
        self.next_sample_time = ((2048 - freq_x) as f32) / 65536.0 / 32.0;
    }

    pub fn update_len(&mut self) {
        self.out_samples_limited = self.nrx4 & (1 << 6) != 0;
        if self.out_samples_limited {
            self.out_sample_count = ((256 - self.nrx1 as u32) as f32 *
                                    (44100.0 / 256.0)) as usize;
        }
    }

    pub fn update_vol(&mut self) {
        self.next_vol =
            match (self.nrx2 >> 5) & 0x03 {
                0 =>  0.0,
                1 => -1.0,
                2 => -0.5,
                3 => -0.25,
                _ => unreachable!(),
            };
    }

    pub fn pull_regs<S: GlobalAudioState>(&mut self, gas: &S) {
        if self.sample_i == 0 {
            for i in 0..16 {
                self.samples[i] = gas.wave_sample(i);
            }
            self.vol = self.next_vol;
            self.sample_time = self.next_sample_time;
        }
    }

    pub fn initialize<S: GlobalAudioState>(&mut self, gas: &mut S) {
        self.update_len();
        self.update_freq();
        self.update_vol();

        self.set_enabled(gas, true);
    }

    pub fn get_sample<S: GlobalAudioState>(&mut self, gas: &mut S) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.soft_stopped && self.sample_i == 0 {
            self.set_enabled(gas, false);
            return 0.0;
        }

        if self.out_samples_limited {
            if self.out_sample_count == 0 {
                if self.sample_i == 0 {
                    self.set_enabled(gas, false);
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

            self.pull_regs(gas);

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
pub struct Noise {
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
    pub fn new(channel: usize) -> Self {
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

    pub fn set_nrx1(&mut self, val: u8) {
        self.nrx1 = val;
        self.update_len();
    }

    pub fn set_nrx2(&mut self, val: u8) {
        self.nrx2 = val;
        self.update_envelope();
    }

    pub fn set_nrx3(&mut self, val: u8) {
        self.nrx3 = val;
        self.update_freq();
    }

    pub fn set_nrx4<S: GlobalAudioState>(&mut self, val: u8, gas: &mut S) {
        self.nrx4 = val;

        self.samples_limited = val & (1 << 6) != 0;

        if val & 0x80 != 0 {
            self.initialize(gas);
        }
    }

    /// `enable_ext` marks this channel as enabled/disabled in NR52.  It takes the channel index
    /// and whether it should be enabled or not.
    pub fn set_enabled<S: GlobalAudioState>(&mut self, gas: &mut S, enabled: bool) {
        self.enabled = enabled;
        gas.enable_channel(self.channel, enabled);
    }

    pub fn update_freq(&mut self) {
        let mut r = self.nrx3 & 0x07;
        let s = self.nrx3 >> 4;

        if r == 0 {
            r = 1;
        } else {
            r *= 2;
        }

        self.shift_time = (r as f32 * ((s + 1) as f32).exp2()) / 1048576.0;
    }

    pub fn update_len(&mut self) {
        self.samples_limited = self.nrx4 & (1 << 6) != 0;
        if self.samples_limited {
            let x = self.nrx1 & 0x3f;
            self.sample_count = ((64 - x) as f32 * (44100.0 / 256.0)) as usize;
        }
    }

    pub fn update_envelope(&mut self) {
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

    pub fn initialize<S: GlobalAudioState>(&mut self, gas: &mut S) {
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

        self.set_enabled(gas, true);
    }

    pub fn shift(&mut self) {
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

    pub fn get_raw_sample<S: GlobalAudioState>(&mut self, gas: &mut S) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        if self.samples_limited {
            if self.sample_count == 0 {
                self.set_enabled(gas, false);
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

    pub fn get_sample<S: GlobalAudioState>(&mut self, gas: &mut S) -> f32 {
        let raw_sample = self.get_raw_sample(gas);
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
    pub outbuf: Arc<Mutex<Vec<f32>>>,
    #[savestate(skip)]
    pub intbuf: Vec<f32>,
    #[savestate(skip)]
    pub outbuf_done: Receiver<usize>,
    #[savestate(skip)]
    pub outbuf_done_handout: Option<Sender<usize>>,

    pub ibuf_i: usize,
    #[savestate(skip)]
    pub obuf_i: usize,
    pub ibuf_i_cycles: f32,
    pub shared: SharedState,

    pub ch1: ToneSweep,
    pub ch2: ToneSweep,
    pub ch3: Wave,
    pub ch4: Noise,

    pub ch3_l: bool,
    pub ch3_r: bool,

    #[savestate(skip)]
    pub postprocess: bool,

    #[savestate(skip)]
    pub last_raw_sample: (f32, f32),
    #[savestate(skip)]
    pub velocity: (f32, f32),
    #[savestate(skip)]
    pub position: (f32, f32),
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

    pub fn reset(&mut self) {
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

    fn gen_one_frame<S: GlobalAudioState>(&mut self, gas: &mut S) -> (f32, f32) {
        let ch1 = self.ch1.get_sample(gas);
        let ch2 = self.ch2.get_sample(gas);
        let ch3 = self.ch3.get_sample(gas);
        let ch4 = self.ch4.get_sample(gas);

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
    pub fn add_cycles<S: GlobalAudioState>(
        &mut self,
        gas: &mut S,
        cycles: u32,
        realtime: bool,
    ) {
        self.ibuf_i_cycles += cycles as f32;

        while self.ibuf_i_cycles >= (2097152.0 / 44100.0) {
            let (l, r) = self.gen_one_frame(gas);

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
    pub fn fill_outbuf<S: GlobalAudioState>(&mut self, gas: &mut S, output: &mut [f32]) {
        for i in 0..(output.len() / 2) {
            let (l, r) = self.gen_one_frame(gas);

            output[i * 2] = l;
            output[i * 2 + 1] = r;
        }
    }

    pub fn set_postprocessing(&mut self, postprocess: bool) {
        self.postprocess = postprocess;
    }
}
