#![feature(box_syntax)]
// FIXME: These two are here because this file is effectively empty on anything but wasm32.  We
// don't want any messages to occur about this, ideally we wouldn't even build a library on
// any platform but wasm, but seems like cargo is really really keen on always building src/lib.rs.
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate savestate_derive;

#[cfg_attr(not(target_os = "linux"), path = "address_space_generic.rs")]
mod address_space;
mod cpu;
mod io;
mod rom;
mod sgb;
mod system_state;
mod ui;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use address_space::AddressSpace;
use io::serial::SerialConnParam;
use system_state::{System, SystemState};
use ui::UI;


#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct XGBCSystem {
    sys: Box<System>,
}


#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl XGBCSystem {
    pub fn new(buf: Vec<u8>) -> Self {
        console_error_panic_hook::set_once();

        let mut addr_space = box AddressSpace::new(buf);
        let mut sys_params = rom::load_rom(addr_space.as_mut());
        sys_params.serial_conn_param = SerialConnParam::Disabled;

        let mut ui = UI::new(&sys_params.cartridge_name);

        let system_state = box SystemState::new(addr_space, sys_params, &mut ui);
        let system = box System::new(system_state, ui, "".into());

        XGBCSystem {
            sys: system,
        }
    }

    pub fn main_loop_iter(&mut self) {
        self.sys.main_loop(true);
    }

    pub fn get_sound_ringbuf(&self) -> *const f32 {
        self.sys.ui.get_sound_ringbuf().map(|s| &s[0] as *const f32).unwrap_or_else(std::ptr::null)
    }

    pub fn get_sound_ringbuf_length(&self) -> usize {
        self.sys.ui.get_sound_ringbuf().map(|s| s.len()).unwrap_or(0)
    }

    pub fn get_sound_ringbuf_ptrs(&mut self) -> *mut u32 {
        self.sys.ui.get_sound_ringbuf_ptrs().map(|s| &mut s[0] as *mut u32).unwrap_or_else(std::ptr::null_mut)
    }
}
