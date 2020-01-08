use savestate::SaveState;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::address_space::AddressSpace;
use crate::cpu::CPU;
use crate::io;
use crate::io::keypad::KeypadState;
use crate::io::lcd::DisplayState;
use crate::io::sound::SoundState;
use crate::io::timer::TimerState;
use crate::ui::UI;

#[allow(dead_code)]
pub enum IOReg {
    P1      = 0x00,
    SB      = 0x01,
    SC      = 0x02,
    DIV     = 0x04,
    TIMA    = 0x05,
    TMA     = 0x06,
    TAC     = 0x07,
    IF      = 0x0f,
    NR10    = 0x10,
    NR11    = 0x11,
    NR12    = 0x12,
    NR13    = 0x13,
    NR14    = 0x14,
    NR20    = 0x15,
    NR21    = 0x16,
    NR22    = 0x17,
    NR23    = 0x18,
    NR24    = 0x19,
    NR30    = 0x1a,
    NR31    = 0x1b,
    NR32    = 0x1c,
    NR33    = 0x1d,
    NR34    = 0x1e,
    NR40    = 0x1f,
    NR41    = 0x20,
    NR42    = 0x21,
    NR43    = 0x22,
    NR44    = 0x23,
    NR50    = 0x24,
    NR51    = 0x25,
    NR52    = 0x26,
    WAVE00  = 0x30,
    WAVE02  = 0x31,
    WAVE04  = 0x32,
    WAVE06  = 0x33,
    WAVE08  = 0x34,
    WAVE0a  = 0x35,
    WAVE0c  = 0x36,
    WAVE0e  = 0x37,
    WAVE10  = 0x38,
    WAVE12  = 0x39,
    WAVE14  = 0x3a,
    WAVE16  = 0x3b,
    WAVE18  = 0x3c,
    WAVE1a  = 0x3d,
    WAVE1c  = 0x3e,
    WAVE1e  = 0x3f,
    LCDC    = 0x40,
    STAT    = 0x41,
    SCY     = 0x42,
    SCX     = 0x43,
    LY      = 0x44,
    LYC     = 0x45,
    DMA     = 0x46,
    BGP     = 0x47,
    OBP0    = 0x48,
    OBP1    = 0x49,
    WY      = 0x4a,
    WX      = 0x4b,
    KEY1    = 0x4d,
    VBK     = 0x4f,
    HDMA1   = 0x51,
    HDMA2   = 0x52,
    HDMA3   = 0x53,
    HDMA4   = 0x54,
    HDMA5   = 0x55,
    RP      = 0x56,
    BCPS    = 0x68,
    BCPD    = 0x69,
    OCPS    = 0x6a,
    OCPD    = 0x6b,
    SVBK    = 0x70,
    IE      = 0xff,
}

pub enum UIScancode {
    X,
    Z,

    Shift,
    Alt,
    Control,

    Space,
    Return,
    Backspace,

    Left,
    Right,
    Up,
    Down,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

pub enum UIEvent {
    Quit,
    Key { key: UIScancode, down: bool },
}

pub struct SystemParams {
    pub cgb: bool,
}

pub struct AudioOutputParams {
    pub freq: usize,
    pub channels: usize,

    pub buf: Arc<Mutex<Vec<f32>>>,
    pub buf_done: Sender<()>,
}

struct KeyboardState {
    shift: bool,
    alt: bool,
    control: bool,
}

#[derive(SaveState)]
pub struct System {
    pub sys_state: SystemState,
    pub cpu: CPU,

    #[savestate(skip)]
    pub ui: UI,

    #[savestate(skip)]
    keyboard_state: KeyboardState,
    #[savestate(skip)]
    base_path: String,
}

#[derive(SaveState)]
pub struct SystemState {
    pub addr_space: AddressSpace,

    pub cgb: bool,
    pub ints_enabled: bool,
    pub double_speed: bool,
    #[savestate(skip)]
    pub realtime: bool,
    pub vblanked: bool,

    pub display: DisplayState,
    pub keypad: KeypadState,
    pub sound: SoundState,
    pub timer: TimerState,
}


impl System {
    pub fn new(mut sys_state: SystemState, mut ui: UI, base_path: String)
        -> Self
    {
        let cpu = CPU::new(sys_state.cgb);

        ui.setup_audio(sys_state.sound.get_audio_params());

        Self {
            sys_state: sys_state,
            cpu: cpu,

            ui: ui,

            keyboard_state: KeyboardState {
                shift: false,
                alt: false,
                control: false,
            },
            base_path: base_path,
        }
    }

    fn fkey(&mut self, key: usize, down: bool) {
        if !down || self.keyboard_state.alt || self.keyboard_state.control {
            return;
        }

        let fname = format!("{}.ss{}", self.base_path, key);

        let mut opts = std::fs::OpenOptions::new();
        if self.keyboard_state.shift {
            opts.write(true).create(true);
        } else {
            opts.read(true);
        }

        let mut file =
            match opts.open(&fname) {
                Ok(f) => f,
                Err(e) => {
                    println!("Failed to open {}: {}", fname, e);
                    return;
                }
            };

        if self.keyboard_state.shift {
            self.export(&mut file);
            println!("Exported save state {} to {}", key + 1, fname);
        } else {
            self.import(&mut file);
            println!("Imported save state {} from {}", key + 1, fname);
        }
    }

    pub fn exec(&mut self) {
        let cycles = self.cpu.exec(&mut self.sys_state);
        self.sys_state.add_cycles(cycles);

        if self.sys_state.vblanked {
            self.sys_state.vblanked = false;

            self.ui.present_frame(&self.sys_state.display.lcd_pixels);

            while let Some(evt) = self.ui.poll_event() {
                match evt {
                    UIEvent::Quit => {
                        std::process::exit(0);
                    },

                    UIEvent::Key { key, down } => {
                        match key {
                            UIScancode::Space => {
                                self.sys_state.realtime = !down;
                            },

                            UIScancode::Shift => {
                                self.keyboard_state.shift = down;
                            },

                            UIScancode::Alt => {
                                self.keyboard_state.alt = down;
                            },

                            UIScancode::Control => {
                                self.keyboard_state.control = down;
                            },

                            UIScancode::F1  => self.fkey(0, down),
                            UIScancode::F2  => self.fkey(1, down),
                            UIScancode::F3  => self.fkey(2, down),
                            UIScancode::F4  => self.fkey(3, down),
                            UIScancode::F5  => self.fkey(4, down),
                            UIScancode::F6  => self.fkey(5, down),
                            UIScancode::F7  => self.fkey(6, down),
                            UIScancode::F8  => self.fkey(7, down),
                            UIScancode::F9  => self.fkey(8, down),
                            UIScancode::F10 => self.fkey(9, down),
                            UIScancode::F11 => self.fkey(10, down),
                            UIScancode::F12 => self.fkey(11, down),

                            _ => self.sys_state.keypad.key_event(key, down),
                        }
                    },
                }
            }
        }
    }

    pub fn main_loop(&mut self) {
        loop {
            self.exec();
        }
    }
}

impl SystemState {
    pub fn new(addr_space: AddressSpace, params: SystemParams) -> Self {
        let mut state = Self {
            addr_space: addr_space,

            cgb: params.cgb,
            ints_enabled: true,
            double_speed: false,
            realtime: true,
            vblanked: false,

            display: DisplayState::new(),
            keypad: KeypadState::new(),
            sound: SoundState::new(),
            timer: TimerState::new(),
        };

        DisplayState::init_system_state(&mut state);
        KeypadState::init_system_state(&mut state);
        io::init_dma(&mut state);

        state
    }

    pub fn add_cycles(&mut self, count: u32) {
        let dcycles =
            if self.double_speed {
                count
            } else {
                count * 2
            };

        io::lcd::add_cycles(self, dcycles);
        self.sound.add_cycles(dcycles, self.realtime);
        io::timer::add_cycles(self, count);
    }
}
