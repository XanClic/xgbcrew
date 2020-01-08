#[macro_use] extern crate serde_derive;
#[macro_use] extern crate savestate_derive;

mod address_space;
mod cpu;
mod io;
mod rom;
mod system_state;

use std::env;
use std::process::exit;
use regex::Regex;

use address_space::AddressSpace;
use system_state::SystemState;


fn main() {
    let argv: Vec<String> = env::args().collect();

    if argv.len() < 2 {
        eprintln!("Usage: {} <ROM> [RAM]", argv[0]);
        exit(1);
    }

    let rom_path = argv[1].clone();
    let ram_path = match argv.get(2) {
        Some(p) => p.clone(),
        None => {
            let regex = Regex::new(r"\.?[^./]*$").unwrap();
            let replaced = regex.replace(&rom_path, ".sav");

            String::from(replaced)
        },
    };

    let addr_space = AddressSpace::new(&rom_path, &ram_path);
    let mut state = SystemState::new(addr_space);

    rom::load_rom(&mut state);

    let mut system = state.into_system();

    loop {
        system.exec();
    }
}
