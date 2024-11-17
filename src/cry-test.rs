#![allow(dead_code)]

#[macro_use] extern crate savestate_derive;

mod cries;
#[path = "io/sound/functionality.rs"] mod sound;

use cries::*;
use sound::{GlobalAudioState, SoundState};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

fn main() {
    let sdl = sdl2::init().unwrap();
    let audio = sdl.audio().unwrap();

    let sss = SimulatedSystemState::default();
    let mut sound_state = sound::SoundState::new();

    let params = sound_state.get_audio_params();
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

    let sdl_adev = audio.open_playback(
        None,
        &sound_spec,
        adev_obj_gen,
    );
    let sdl_adev = sdl_adev.unwrap();

    sdl_adev.resume();

    let mut psound = PokemonSound::new(sound_state, sss);

    for mon in &[LITWICK, LAMPENT, CHANDELURE] {
        println!("{}", mon.3);
        psound.mon(mon);
        println!("  {} s elapsed", psound.total_cycles as f32 / 2097152.0);

        psound.sound.add_cycles(&mut psound.sys, 2097152, true);
        psound.total_cycles = 0;
    }
}

#[derive(Clone, Copy, Debug)]
enum Instruction {
    // a, b, c, d
    DutyCyclePattern(u8, u8, u8, u8),
    // length, volume, fade, period
    SquareNote(usize, u8, i8, u16),
    // pace, step
    PitchSweep(u8, i8),
    // length, volume, fade, nrx3
    NoiseNote(usize, u8, i8, u8),
    // offset
    PitchOffset(i16),
    // a
    DutyCycle(u8),
}

struct PokemonSound {
    sound: SoundState,
    sys: SimulatedSystemState,

    pitch_offset: [i16; 4],
    tempo: usize,
    total_cycles: u32,
    in_frame_cycles: u32,

    duty_cycle_patterns: [Option<u8>; 2],
    insns: [VecDeque<Instruction>; 4],
    insn_delay: [usize; 4],
    channel_on: [bool; 4],
}

// Could be either of 2^21/(19200/60)/(12*4)
// (doc says 19200 / tempo is the BPM, this assumes that the default note length is 12, and that
// BPM means 4)
// Or 35112/256 (frame length divided into 256 steps)
const TICK_CYCLES: u32 = 137;

impl PokemonSound {
    fn new(sound: SoundState, sys: SimulatedSystemState) -> Self {
        PokemonSound {
            sound,
            sys,
            pitch_offset: [0; 4],
            tempo: 128,
            total_cycles: 0,
            in_frame_cycles: 0,
            duty_cycle_patterns: [Some(0b10101010); 2],
            insns: [const { VecDeque::new() }; 4],
            insn_delay: [0; 4],
            channel_on: [false; 4],
        }
    }

    fn set_pitch_offset(&mut self, ofs: i16) {
        for i in 0..4 {
            self.pitch_offset[i] = ofs;
        }
    }

    fn set_channel_pitch_offset(&mut self, channel: usize, ofs: i16) {
        self.pitch_offset[channel - 1] += ofs;
    }

    fn set_tempo(&mut self, tempo: usize) {
        self.tempo = tempo;
    }

    fn init_channel(&mut self, channel: usize) {
        match channel {
            1 => self.sound.ch1.initialize(&mut self.sys),
            2 => self.sound.ch2.initialize(&mut self.sys),
            3 => self.sound.ch3.initialize(&mut self.sys),
            4 => self.sound.ch4.initialize(&mut self.sys),
            _ => panic!("No such channel"),
        }
    }

    fn off_channel(&mut self, channel: usize) {
        match channel {
            1 => self.sound.ch1.set_enabled(&mut self.sys, false),
            2 => self.sound.ch2.set_enabled(&mut self.sys, false),
            3 => self.sound.ch3.set_enabled(&mut self.sys, false),
            4 => self.sound.ch4.set_enabled(&mut self.sys, false),
            _ => panic!("No such channel"),
        }
    }

    fn tone_channel(&mut self, channel: usize) -> &mut sound::ToneSweep {
        match channel {
            1 => &mut self.sound.ch1,
            2 => &mut self.sound.ch2,
            _ => panic!("Not a tone channel"),
        }
    }

    fn do_tone_duty_cycle(&mut self, channel: usize, duty: u8) {
        self.tone_channel(channel).set_nrx1(duty << 6);
    }

    fn tone_duty_cycle_pattern(&mut self, channel: usize, cycle: u8) {
        self.duty_cycle_patterns[channel - 1] = Some(cycle);
        self.do_tone_duty_cycle(channel, cycle >> 6);
    }

    fn tone_duty_cycle(&mut self, channel: usize, duty: u8) {
        self.duty_cycle_patterns[channel - 1].take();
        self.do_tone_duty_cycle(channel, duty);
    }

    fn tone_volume(&mut self, channel: usize, volume: u8, fade: i8) {
        let nrx2 = if fade < 0 {
            (volume << 4) | (1 << 3) | (-fade as u8)
        } else {
            (volume << 4) | (0 << 3) | (fade as u8)
        };
        self.tone_channel(channel).set_nrx2(nrx2);
    }

    fn tone_freq(&mut self, channel: usize, period: u16) {
        let ch = match channel {
            1 => &mut self.sound.ch1,
            2 => &mut self.sound.ch2,
            _ => panic!("Not a tone channel"),
        };
        let period = period.wrapping_add(self.pitch_offset[channel - 1] as u16);
        ch.set_nrx3(period as u8);
        ch.set_nrx4((period >> 8) as u8, &mut self.sys);
    }

    fn tone_pitch_sweep(&mut self, pace: u8, step: i8) {
        let nr10 = if step < 0 {
            (pace << 4) | (1 << 3) | (-step as u8)
        } else {
            (pace << 4) | (0 << 3) | (step as u8)
        };
        self.sound.ch1.set_nrx0(nr10);
    }

    fn noise_volume(&mut self, volume: u8, fade: i8) {
        let nr42 = if fade < 0 {
            (volume << 4) | (1 << 3) | (-fade as u8)
        } else {
            (volume << 4) | (0 << 3) | (fade as u8)
        };
        self.sound.ch4.set_nrx2(nr42);
    }

    fn noise_freq(&mut self, nrx3: u8) {
        let nrx3 = nrx3.wrapping_add(self.pitch_offset[3] as u8);
        self.sound.ch4.set_nrx3(nrx3);
        self.sound.ch4.initialize(&mut self.sys);
    }

    fn tick(&mut self) -> bool {
        self.total_cycles += TICK_CYCLES;
        self.sound.add_cycles(&mut self.sys, TICK_CYCLES, true);

        self.in_frame_cycles += TICK_CYCLES;
        if let Some(new) = self.in_frame_cycles.checked_sub(35112) {
            self.duty_cycle_pattern_tick();
            self.in_frame_cycles = new;
            true
        } else {
            false
        }
    }

    fn duty_cycle_pattern_tick(&mut self) {
        for i in 1..=2 {
            if let Some(duty) = self.duty_cycle_patterns[i - 1] {
                self.tone_duty_cycle_pattern(i, duty.rotate_left(2));
            }
        }
    }

    fn channel(&mut self, channel: usize, insns: &[Instruction]) {
        self.channel_on[channel - 1] = !insns.is_empty();
        if self.channel_on[channel - 1] {
            self.init_channel(channel);
        } else {
            self.off_channel(channel);
        }

        self.insn_delay[channel - 1] = 0;
        self.insns[channel - 1] = VecDeque::from_iter(insns.into_iter().copied());
    }

    fn run_instructions(&mut self) {
        let mut may_fetch_new = true;

        loop {
            for i in 1..=4 {
                if !self.channel_on[i - 1] {
                    continue;
                }

                while self.insn_delay[i - 1] == 0 && may_fetch_new {
                    let Some(insn) = self.insns[i - 1].pop_front() else {
                        self.channel_on[i - 1] = false;
                        self.off_channel(i);
                        break;
                    };

                    match insn {
                        Instruction::DutyCyclePattern(a, b, c, d) => {
                            self.tone_duty_cycle_pattern(i, (a << 6) | (b << 4) | (c << 2) | d);
                        }

                        Instruction::DutyCycle(a) => self.tone_duty_cycle(i, a),

                        Instruction::SquareNote(length, volume, fade, period) => {
                            self.tone_volume(i, volume, fade);
                            self.tone_freq(i, period);
                            self.insn_delay[i - 1] = length * self.tempo;
                        }

                        Instruction::PitchSweep(pace, step) => {
                            if i != 1 {
                                panic!("Not a sweep channel");
                            }

                            self.tone_pitch_sweep(pace, step);
                        }

                        Instruction::PitchOffset(ofs) => {
                            self.set_channel_pitch_offset(i, ofs);
                        }

                        Instruction::NoiseNote(length, volume, fade, nrx3) => {
                            if i != 4 {
                                panic!("Not a noise channel");
                            }

                            self.noise_volume(volume, fade);
                            self.noise_freq(nrx3);
                            self.insn_delay[3] = length * 256;
                        }
                    }
                }

                self.insn_delay[i - 1] = self.insn_delay[i - 1].saturating_sub(1);
            }

            let any = self.channel_on.iter().any(|x| *x);
            if any {
                may_fetch_new = self.tick();
            } else {
                break;
            }
        }
    }

    fn mon(&mut self, cry: &MonCry) {
        self.set_pitch_offset(cry.0);
        self.set_tempo(cry.1);

        self.channel(1, cry.2.0);
        self.channel(2, cry.2.1);
        self.channel(3, cry.2.2);
        self.channel(4, cry.2.3);

        self.run_instructions();
    }
}

#[derive(Default)]
struct SimulatedSystemState {
    audio_channels_enabled: [bool; 4],
    wave_samples: [u8; 16],
}

impl GlobalAudioState for SimulatedSystemState {
    fn enable_channel(&mut self, channel: usize, enabled: bool) {
        self.audio_channels_enabled[channel] = enabled;
    }

    fn wave_sample(&self, i: usize) -> u8 {
        self.wave_samples[i]
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
