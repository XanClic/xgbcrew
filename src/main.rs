#[macro_use] extern crate serde_derive;
#[macro_use] extern crate savestate_derive;

mod address_space;
mod cpu;
mod io;
mod rom;
mod sgb;
mod system_state;
mod ui;

use std::env;
use std::process::exit;
use regex::Regex;

use address_space::AddressSpace;
use system_state::{System, SystemState};
use ui::UI;


fn main() {
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
    let mut system = System::new(system_state, UI::new(), base_path);

    system.main_loop();
}
