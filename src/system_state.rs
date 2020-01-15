use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::address_space::AddressSpace;
use crate::cpu::CPU;
use crate::io;
use crate::io::keypad::{KeypadState, KeypadKey};
use crate::io::lcd::DisplayState;
use crate::io::sound::SoundState;
use crate::io::timer::TimerState;
use crate::sgb::SGBState;
use crate::ui::UI;


const SAVE_STATE_VERSION: u64 = 4;

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

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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

    CA,
    CB,
    CStart,
    CSelect,
    CLeft,
    CRight,
    CUp,
    CDown,
    CSkip,

    CLoad(usize),
    CSave(usize),
}

enum UIAction {
    Key(KeypadKey, bool),

    Skip(bool),

    LoadState(usize),
    SaveState(usize),

    Quit,
}

pub enum UIEvent {
    Quit,
    Key { key: UIScancode, down: bool },
}

pub struct SystemParams {
    pub cgb: bool,
    pub sgb: bool,
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

    #[savestate(skip_if("version >= 1"))]
    pub cgb: bool,
    #[savestate(skip)]
    pub sgb: bool,
    pub ints_enabled: bool,
    pub double_speed: bool,
    #[savestate(skip)]
    pub realtime: bool,
    pub vblanked: bool,

    /* Savestating the border is broken anyway */
    #[savestate(skip)]
    pub enable_sgb_border: bool,

    pub display: DisplayState,
    pub keypad: KeypadState,
    pub sound: SoundState,
    pub timer: TimerState,

    #[savestate(skip_if("version < 1"))]
    pub sgb_state: SGBState,
}


impl System {
    pub fn new(mut sys_state: SystemState, mut ui: UI, base_path: String)
        -> Self
    {
        let cpu = CPU::new(sys_state.cgb, sys_state.sgb);

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

    fn do_save_state(&mut self, index: usize, save: bool) {
        let fname = format!("{}.ss{}", self.base_path, index);

        let mut opts = std::fs::OpenOptions::new();
        if save {
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

        if save {
            savestate::export_root(self, &mut file, SAVE_STATE_VERSION);
            println!("Exported save state {} to {}", index + 1, fname);
        } else {
            savestate::import_root(self, &mut file, SAVE_STATE_VERSION);
            println!("Imported save state {} from {}", index + 1, fname);
        }
    }

    fn fkey(&mut self, index: usize) -> Option<UIAction> {
        if !self.keyboard_state.alt && !self.keyboard_state.control {
            if self.keyboard_state.shift {
                Some(UIAction::SaveState(index))
            } else {
                Some(UIAction::LoadState(index))
            }
        } else {
            None
        }
    }

    fn translate_event(&mut self, event: UIEvent) -> Option<UIAction> {
        if let Some(action) = match event {
            UIEvent::Quit => Some(UIAction::Quit),

            UIEvent::Key { key, down: true } => {
                match key {
                    UIScancode::F1  => self.fkey(0),
                    UIScancode::F2  => self.fkey(1),
                    UIScancode::F3  => self.fkey(2),
                    UIScancode::F4  => self.fkey(3),
                    UIScancode::F5  => self.fkey(4),
                    UIScancode::F6  => self.fkey(5),
                    UIScancode::F7  => self.fkey(6),
                    UIScancode::F8  => self.fkey(7),
                    UIScancode::F9  => self.fkey(8),
                    UIScancode::F10 => self.fkey(9),
                    UIScancode::F11 => self.fkey(10),
                    UIScancode::F12 => self.fkey(11),

                    UIScancode::CLoad(x) => Some(UIAction::LoadState(x)),
                    UIScancode::CSave(x) => Some(UIAction::SaveState(x)),

                    _ => None,
                }
            },

            _ => None,
        }
        {
            return Some(action);
        }

        match event {
            UIEvent::Key { key, down } => {
                match key {
                    UIScancode::Shift => {
                        self.keyboard_state.shift = down;
                        None
                    },

                    UIScancode::Alt => {
                        self.keyboard_state.alt = down;
                        None
                    },

                    UIScancode::Control => {
                        self.keyboard_state.control = down;
                        None
                    },

                    UIScancode::Space | UIScancode::CSkip =>
                        Some(UIAction::Skip(down)),

                    UIScancode::X | UIScancode::CA =>
                        Some(UIAction::Key(KeypadKey::A, down)),

                    UIScancode::Z | UIScancode::CB =>
                        Some(UIAction::Key(KeypadKey::B, down)),

                    UIScancode::Return | UIScancode::CStart =>
                        Some(UIAction::Key(KeypadKey::Start, down)),

                    UIScancode::Backspace | UIScancode::CSelect =>
                        Some(UIAction::Key(KeypadKey::Select, down)),

                    UIScancode::Left | UIScancode::CLeft =>
                        Some(UIAction::Key(KeypadKey::Left, down)),

                    UIScancode::Right | UIScancode::CRight =>
                        Some(UIAction::Key(KeypadKey::Right, down)),

                    UIScancode::Up | UIScancode::CUp =>
                        Some(UIAction::Key(KeypadKey::Up, down)),

                    UIScancode::Down | UIScancode::CDown =>
                        Some(UIAction::Key(KeypadKey::Down, down)),

                    _ => None,
                }
            },

            _ => None,
        }
    }

    fn perform_ui_action(&mut self, action: UIAction) {
        match action {
            UIAction::Key(key, down) =>
                self.sys_state.keypad.key_event(key, down),

            UIAction::Skip(skip) =>
                self.sys_state.realtime = !skip,

            UIAction::LoadState(index) =>
                self.do_save_state(index, false),

            UIAction::SaveState(index) =>
                self.do_save_state(index, true),

            UIAction::Quit =>
                std::process::exit(0),
        }
    }

    fn poll_events(&mut self) {
        while let Some(evt) = self.ui.poll_event() {
            if let Some(action) = self.translate_event(evt) {
                self.perform_ui_action(action);
            }
        }
    }

    pub fn exec(&mut self) {
        let cycles = self.cpu.exec(&mut self.sys_state);
        self.sys_state.add_cycles(cycles);

        if self.sys_state.vblanked {
            self.sys_state.vblanked = false;

            self.ui.present_frame(&self.sys_state.display.lcd_pixels);

            if self.sys_state.enable_sgb_border {
                self.sys_state.enable_sgb_border = false;
                self.ui.enable_sgb_border();

                self.ui.set_sgb_border(&self.sys_state.sgb_state.border_pixels);
            }

            self.ui.rumble(self.sys_state.addr_space.cartridge.rumble_state);

            self.poll_events();
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
            sgb: params.sgb && !params.cgb,
            ints_enabled: true,
            double_speed: false,
            realtime: true,
            vblanked: false,
            enable_sgb_border: false,

            display: DisplayState::new(),
            keypad: KeypadState::new(),
            sound: SoundState::new(),
            timer: TimerState::new(),

            sgb_state: SGBState::new(),
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
