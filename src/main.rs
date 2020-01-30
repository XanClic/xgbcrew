#[macro_use] extern crate serde_derive;
#[macro_use] extern crate savestate_derive;

#[cfg_attr(not(target_os = "linux"), path = "address_space_generic.rs")]
mod address_space;
mod cpu;
mod io;
mod rom;
mod sc;
mod sgb;
mod system_state;
mod ui;

use std::env;
use std::process::exit;
use regex::Regex;

use address_space::AddressSpace;
use system_state::{System, SystemState};
use ui::UI;
use ui::sdl::SDLUI;


fn real_main() {
    let argv: Vec<String> = env::args().collect();

    if argv.len() < 2 {
        eprintln!("Usage: {} <ROM> [RAM]", argv[0]);
        exit(1);
    }

    let rom_path = argv[1].clone();

    let regex = Regex::new(r"\.?[^./]*$").unwrap();
    let base_path = String::from(regex.replace(&rom_path, ""));

    let ram_path = match argv.get(2) {
        Some(p) => p.clone(),
        None => format!("{}.sav", base_path),
    };

    let mut addr_space = AddressSpace::new(&rom_path, &ram_path);
    let sys_params = rom::load_rom(&mut addr_space);

    let system_state = SystemState::new(addr_space, sys_params);
    let ui = UI::new(SDLUI::new());
    let mut system = System::new(system_state, ui, base_path);

    system.main_loop();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    real_main();
}

#[cfg(target_os = "windows")]
fn main() {
    let main_thread = std::thread::Builder::new().name(String::from("main"))
                                                 .stack_size(4 << 20)
                                                 .spawn(|| { real_main() })
                                                 .unwrap();

    if let Err(e) = main_thread.join() {
        if let Some(s) = e.downcast_ref::<String>() {
            panic!("{}", s);
        } else if let Some(s) = e.downcast_ref::<&str>() {
            panic!("{}", s);
        } else {
            panic!("{:?}", e);
        }
    }
}
