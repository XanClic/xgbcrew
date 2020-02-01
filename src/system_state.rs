use crate::address_space::AddressSpace;
use crate::cpu::CPU;
use crate::io;
use crate::io::keypad::KeypadState;
use crate::io::lcd::DisplayState;
use crate::io::sound::SoundState;
use crate::io::timer::TimerState;
use crate::sgb::SGBState;
use crate::ui::{UI, UIAction, UIEvent};


const SAVE_STATE_VERSION: u64 = 7;

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

pub struct SystemParams {
    pub cgb: bool,
    pub sgb: bool,
}

#[derive(SaveState)]
pub struct System {
    pub sys_state: SystemState,
    pub cpu: CPU,

    #[savestate(skip)]
    pub ui: UI,

    #[savestate(skip)]
    base_path: String,

    #[savestate(skip)]
    paused: bool,
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

    #[savestate(skip)]
    sound_postprocess: bool,

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

            base_path: base_path,

            paused: false,
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
                    let msg = format!("Failed to load SS {} ({}):\n{}",
                                      index + 1, fname, e);
                    self.ui.osd_message(msg);
                    return;
                }
            };

        if save {
            savestate::export_root(self, &mut file, SAVE_STATE_VERSION);
            self.ui.osd_message(format!("Created save state {}", index + 1));
        } else {
            savestate::import_root(self, &mut file, SAVE_STATE_VERSION);
            self.sys_state.keypad.post_import(&mut self.sys_state.addr_space);
            self.ui.osd_message(format!("Loaded save state {}", index + 1));
        }
    }

    fn perform_ui_action(&mut self, action: UIAction) {
        match action {
            UIAction::Key(key, down) => {
                let addr_space = &mut self.sys_state.addr_space;
                self.sys_state.keypad.key_event(addr_space, key, down);
            },

            UIAction::Skip(skip) =>
                self.sys_state.realtime = !skip,

            UIAction::ToggleAudioPostprocessing => {
                self.sys_state.toggle_sound_postprocess();

                let pp_state =
                    if self.sys_state.sound_postprocess {
                        "enabled"
                    } else {
                        "disabled"
                    };

                self.ui.osd_message(format!("Sound postprocessing {}",
                                            pp_state));
            },

            UIAction::LoadState(index) => {
                self.do_save_state(index, false);
                self.ui.refresh_lcd(&self.sys_state);
            },

            UIAction::SaveState(index) =>
                self.do_save_state(index, true),

            UIAction::ToggleFullscreen =>
                self.ui.toggle_fullscreen(),

            UIAction::TogglePause => {
                self.paused = !self.paused;
                self.ui.set_paused(self.paused);

                if self.paused {
                    self.ui.osd_message(String::from("Paused"));
                } else {
                    self.ui.osd_message(String::from("Resumed"));
                }
            }

            UIAction::Quit =>
                std::process::exit(0),
        }
    }

    fn exec(&mut self) {
        let cycles = self.cpu.exec(&mut self.sys_state);
        self.sys_state.add_cycles(cycles);
    }

    fn get_event(&mut self) -> Option<UIEvent> {
        /* Pausing will cause us to always return Some() until the
         * game is unpaused again.  So until then, we are caught up
         * in the event loop and automatically will not exec anything. */
        if self.paused {
            Some(self.ui.wait_event(&self.sys_state))
        } else {
            self.ui.poll_event()
        }
    }

    fn handle_events(&mut self) {
        self.ui.vblank_events(&self.sys_state);

        if self.sys_state.sgb_state.load_border {
            self.sys_state.sgb_state.load_border = false;
            self.ui.load_sgb_border(&self.sys_state);
        }

        while let Some(evt) = self.get_event() {
            if let Some(action) = self.ui.translate_event(evt) {
                self.perform_ui_action(action);
            }
        }
    }

    pub fn main_loop(&mut self) {
        loop {
            self.exec();
            if self.sys_state.vblanked {
                self.sys_state.vblanked = false;
                self.ui.refresh_lcd(&self.sys_state);
                self.handle_events();
            }
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

            sound_postprocess: false,

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
        self.sound.add_cycles(&mut self.addr_space, dcycles, self.realtime);
        io::timer::add_cycles(self, count);
    }

    fn toggle_sound_postprocess(&mut self) {
        self.sound_postprocess = !self.sound_postprocess;
        self.sound.set_postprocessing(self.sound_postprocess);
    }
}
