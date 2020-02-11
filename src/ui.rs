pub mod sc;
pub mod sdl;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use crate::io::keypad::KeypadKey;
use crate::system_state::SystemState;
use sdl::SDLUI;
use sc::SC;


#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
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
    CX,
    CY,
    CLeft,
    CRight,
    CUp,
    CDown,
    CLBump,
    CRBump,
    CLTrigger,
    CRTrigger,
    CLGrip,
    CRGrip,
    CPrevious,
    CNext,
    CAction,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
enum UIInputEdge {
    Down,
    Up,
}

impl Default for UIInputEdge {
    fn default() -> Self {
        UIInputEdge::Down
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
struct UIInput {
    scancode: UIScancode,

    #[serde(default)]
    shift: bool,
    #[serde(default)]
    alt: bool,
    #[serde(default)]
    control: bool,
    #[serde(default)]
    edge: UIInputEdge,
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
struct UIMap {
    #[serde(flatten)]
    input: UIInput,

    action: UIAction,
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
    sc: Option<SC>,

    keyboard_state: KeyboardState,
    fullscreen: bool,
    paused: bool,

    input_map: HashMap<UIInput, UIAction>,
}


macro_rules! binding {
    ($im:ident, $sc:ident, $s:ident, $a:ident, $c:ident, $e:ident,
     $act:expr) =>
    {
        $im.insert(UIInput { scancode: UIScancode::$sc,
                             shift: $s, alt: $a, control: $c,
                             edge: UIInputEdge::$e },
                   $act);
    };

    ($im:ident, $sc:ident, $s:ident, $a:ident, $c:ident, $kpk:ident) => {
        $im.insert(UIInput { scancode: UIScancode::$sc,
                             shift: $s, alt: $a, control: $c,
                             edge: UIInputEdge::Down },
                   UIAction::Key(KeypadKey::$kpk, true));

        $im.insert(UIInput { scancode: UIScancode::$sc,
                             shift: $s, alt: $a, control: $c,
                             edge: UIInputEdge::Up },
                   UIAction::Key(KeypadKey::$kpk, false));
    };
}

impl UI {
    pub fn new(cart_name: &String) -> Self {
        let mut frontend = SDLUI::new();

        let sc = match SC::new() {
            Ok(sc) => sc,
            Err(msg) => {
                let d = std::time::Duration::from_secs(5);
                frontend.osd_timed_message(msg, d);
                None
            },
        };

        Self {
            frontend: frontend,
            sc: sc,

            keyboard_state: KeyboardState {
                shift: false,
                alt: false,
                control: false,
            },

            fullscreen: false,
            paused: false,

            input_map: Self::load_input_mapping(cart_name),
        }
    }

    fn load_input_mapping(cart_name: &String) -> HashMap::<UIInput, UIAction> {
        let mut opts = std::fs::OpenOptions::new();
        opts.read(true);

        let map_file =
            match opts.open("input-map.json") {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to load input-map.json: {}", e);
                    return Self::default_input_mapping();
                }
            };

        type InputMaps = HashMap::<String, Vec::<UIMap>>;
        let mut cfg: InputMaps = serde_json::from_reader(map_file).unwrap();

        if let Some(map) = cfg.remove(cart_name) {
            Self::translate_input_mapping(map)
        } else if let Some(map) = cfg.remove("default") {
            Self::translate_input_mapping(map)
        } else {
            Self::default_input_mapping()
        }
    }

    fn translate_input_mapping(json_map: Vec::<UIMap>)
        -> HashMap::<UIInput, UIAction>
    {
        let mut im = HashMap::new();

        for uim in json_map {
            im.insert(uim.input, uim.action);
        }

        im
    }

    fn default_input_mapping() -> HashMap::<UIInput, UIAction> {
        let mut im = HashMap::new();

        Self::default_keyboard_mapping(&mut im);
        Self::default_controller_mapping(&mut im);

        im
    }

    fn default_keyboard_mapping(im: &mut HashMap::<UIInput, UIAction>) {
        binding!(im, X, false, false, false, A);
        binding!(im, Z, false, false, false, B);
        binding!(im, Return, false, false, false, Start);
        binding!(im, Backspace, false, false, false, Select);

        binding!(im, Left, false, false, false, Left);
        binding!(im, Right, false, false, false, Right);
        binding!(im, Up, false, false, false, Up);
        binding!(im, Down, false, false, false, Down);

        binding!(im, Space, false, false, false, Down, UIAction::Skip(true));
        binding!(im, Space, false, false, false, Up, UIAction::Skip(false));

        binding!(im, P, false, false, false, Down, UIAction::TogglePause);

        binding!(im, F9, false, false, false, Down,
                 UIAction::ToggleAudioPostprocessing);

        binding!(im, F11, false, false, false, Down,
                 UIAction::ToggleFullscreen);

        binding!(im, F1, false, false, false, Down, UIAction::LoadState(0));
        binding!(im, F2, false, false, false, Down, UIAction::LoadState(1));
        binding!(im, F3, false, false, false, Down, UIAction::LoadState(2));
        binding!(im, F4, false, false, false, Down, UIAction::LoadState(3));
        binding!(im, F5, false, false, false, Down, UIAction::LoadState(4));
        binding!(im, F6, false, false, false, Down, UIAction::LoadState(5));
        binding!(im, F7, false, false, false, Down, UIAction::LoadState(6));
        binding!(im, F8, false, false, false, Down, UIAction::LoadState(7));

        binding!(im, F1, true, false, false, Down, UIAction::SaveState(0));
        binding!(im, F2, true, false, false, Down, UIAction::SaveState(1));
        binding!(im, F3, true, false, false, Down, UIAction::SaveState(2));
        binding!(im, F4, true, false, false, Down, UIAction::SaveState(3));
        binding!(im, F5, true, false, false, Down, UIAction::SaveState(4));
        binding!(im, F6, true, false, false, Down, UIAction::SaveState(5));
        binding!(im, F7, true, false, false, Down, UIAction::SaveState(6));
        binding!(im, F8, true, false, false, Down, UIAction::SaveState(7));
    }

    fn default_controller_mapping(im: &mut HashMap::<UIInput, UIAction>) {
        binding!(im, CB, false, false, false, A);
        binding!(im, CA, false, false, false, B);
        binding!(im, CY, false, false, false, Start);
        binding!(im, CX, false, false, false, Select);

        binding!(im, CLeft, false, false, false, Left);
        binding!(im, CRight, false, false, false, Right);
        binding!(im, CUp, false, false, false, Up);
        binding!(im, CDown, false, false, false, Down);

        binding!(im, CNext, false, false, false, Down, UIAction::Skip(true));
        binding!(im, CNext, false, false, false, Up, UIAction::Skip(false));

        binding!(im, CPrevious, false, false, false, Down,
                 UIAction::TogglePause);

        binding!(im, CAction, false, false, false, Down,
                 UIAction::ToggleFullscreen);

        binding!(im, CLTrigger, false, false, false, Down,
                 UIAction::LoadState(0));
        binding!(im, CRTrigger, false, false, false, Down,
                 UIAction::LoadState(1));

        binding!(im, CLGrip, false, false, false, Down, UIAction::SaveState(0));
        binding!(im, CRGrip, false, false, false, Down, UIAction::SaveState(1));
    }

    pub fn translate_event(&mut self, event: UIEvent) -> Option<UIAction> {
        match event {
            UIEvent::Quit => Some(UIAction::Quit),

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

                    _ => {
                        let edge =
                            if down {
                                UIInputEdge::Down
                            } else {
                                UIInputEdge::Up
                            };

                        let inp = UIInput {
                            scancode: key,

                            shift: self.keyboard_state.shift,
                            alt: self.keyboard_state.alt,
                            control: self.keyboard_state.control,

                            edge: edge,
                        };

                        self.input_map.get(&inp).cloned()
                    },
                }
            },
        }
    }

    fn poll_sc_event(&mut self) -> Option<UIEvent> {
        if let Some(sc) = &mut self.sc {
            sc.poll_event()
        } else {
            None
        }
    }

    pub fn poll_event(&mut self) -> Option<UIEvent> {
        if let Some(evt) = self.poll_sc_event() {
            Some(evt)
        } else {
            self.frontend.poll_event()
        }
    }

    pub fn wait_event(&mut self, sys_state: &SystemState) -> UIEvent {
        let to = std::time::Duration::from_millis(50);

        loop {
            /* TODO: Maybe this shouldn’t be here, but we need it for
             *       OSD messages when paused */
            self.refresh_lcd(sys_state);

            if let Some(sc) = &mut self.sc {
                if let Some(evt) = sc.wait_event(to) {
                    return evt;
                } else if let Some(evt) = self.frontend.poll_event() {
                    return evt;
                }
            } else if let Some(evt) = self.frontend.wait_event(to) {
                return evt;
            }
        }
    }

    pub fn setup_audio(&mut self, params: AudioOutputParams) {
        self.frontend.setup_audio(params)
    }

    pub fn refresh_lcd(&mut self, sys_state: &SystemState) {
        self.frontend.present_frame(&sys_state.display.lcd_pixels);
    }

    pub fn vblank_events(&mut self, sys_state: &SystemState) {
        if let Some(sc) = &mut self.sc {
            sc.rumble(sys_state.addr_space.cartridge.rumble_state &&
                      !self.paused);
        }
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
        self.paused = paused;
        self.frontend.set_paused(paused);

        if !paused {
            if let Some(sc) = &mut self.sc {
                sc.rumble(false);
            }
        }
    }

    pub fn osd_timed_message(&mut self, text: String,
                             duration: std::time::Duration)
    {
        self.frontend.osd_timed_message(text, duration);
    }

    pub fn osd_message(&mut self, text: String) {
        self.osd_timed_message(text, std::time::Duration::from_secs(3));
    }
}
