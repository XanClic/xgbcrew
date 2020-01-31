pub mod sc;
pub mod sdl;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::io::keypad::KeypadKey;
use crate::system_state::SystemState;
use sdl::SDLUI;


#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum UIScancode {
    P,
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
    CPrevious,
    CNext,

    CLoad(usize),
    CSave(usize),
}

pub enum UIAction {
    Key(KeypadKey, bool),

    Skip(bool),
    ToggleAudioPostprocessing,

    LoadState(usize),
    SaveState(usize),

    ToggleFullscreen,
    TogglePause,

    Quit,
}

pub enum UIEvent {
    Quit,
    Key { key: UIScancode, down: bool },
}

pub struct AudioOutputParams {
    pub freq: usize,
    pub channels: usize,

    pub buf: Arc<Mutex<Vec<f32>>>,
    pub buf_step: usize,
    pub buf_done: Sender<usize>,
}

struct KeyboardState {
    shift: bool,
    alt: bool,
    control: bool,
}


pub struct UI {
    frontend: SDLUI,

    keyboard_state: KeyboardState,
    fullscreen: bool,
}


impl UI {
    pub fn new(frontend: SDLUI) -> Self {
        Self {
            frontend: frontend,

            keyboard_state: KeyboardState {
                shift: false,
                alt: false,
                control: false,
            },

            fullscreen: false,
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

    pub fn translate_event(&mut self, event: UIEvent) -> Option<UIAction> {
        if let Some(action) = match event {
            UIEvent::Quit => Some(UIAction::Quit),

            UIEvent::Key { key, down: true } => {
                match key {
                    UIScancode::F1 => self.fkey(0),
                    UIScancode::F2 => self.fkey(1),
                    UIScancode::F3 => self.fkey(2),
                    UIScancode::F4 => self.fkey(3),
                    UIScancode::F5 => self.fkey(4),
                    UIScancode::F6 => self.fkey(5),
                    UIScancode::F7 => self.fkey(6),
                    UIScancode::F8 => self.fkey(7),

                    UIScancode::F9 => Some(UIAction::ToggleAudioPostprocessing),
                    UIScancode::F11 => Some(UIAction::ToggleFullscreen),

                    UIScancode::P | UIScancode::CPrevious =>
                        Some(UIAction::TogglePause),

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

                    UIScancode::Space | UIScancode::CNext =>
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

    pub fn poll_event(&mut self) -> Option<UIEvent> {
        self.frontend.poll_event()
    }

    pub fn wait_event(&mut self) -> UIEvent {
        let to = std::time::Duration::from_millis(50);

        loop {
            if let Some(evt) = self.frontend.wait_event(to) {
                return evt;
            }
        }
    }

    pub fn setup_audio(&mut self, params: AudioOutputParams) {
        self.frontend.setup_audio(params)
    }

    pub fn vblank_events(&mut self, sys_state: &SystemState) {
        self.frontend.present_frame(&sys_state.display.lcd_pixels);
        self.frontend.rumble(sys_state.addr_space.cartridge.rumble_state);
    }

    pub fn load_sgb_border(&mut self, sys_state: &SystemState) {
        self.frontend.enable_sgb_border();
        self.frontend.set_sgb_border(&sys_state.sgb_state.border_pixels);
    }

    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        self.frontend.set_fullscreen(self.fullscreen);
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.frontend.set_paused(paused);
    }
}
