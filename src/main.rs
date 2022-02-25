#![feature(box_syntax)]

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

use std::env;
use std::process::exit;
use regex::Regex;

use address_space::AddressSpace;
use io::serial::SerialConnParam;
use system_state::{System, SystemState};
use ui::UI;


fn main() {
    let argv: Vec<String> = env::args().collect();

    let mut rom_path = None;
    let mut base_path = None;
    let mut ram_path = None;
    let mut scp = SerialConnParam::Disabled;

    let mut arg_iter = argv.iter();
    arg_iter.next(); /* skip argv[0] */

    for arg in arg_iter {
        if arg.starts_with("--") {
            let regex = Regex::new(r"^--([^=]*)(=(.*))?$").unwrap();
            let cap = regex.captures(arg).unwrap();

            if &cap[1] == "serial" {
                if cap.get(3).is_none() ||
                   cap.get(3).unwrap().as_str() == "local-auto"
                {
                    scp = SerialConnParam::LocalAuto;
                } else if cap[3].starts_with("local-shm:") {
                    let pid = cap[3].get(10..).unwrap();
                    scp = SerialConnParam::LocalSHM(pid.parse().unwrap());
                } else if cap[3].starts_with("server:") {
                    let addr = cap[3].get(7..).unwrap();
                    scp = SerialConnParam::Server(String::from(addr));
                } else {
                    scp = SerialConnParam::Client(String::from(&cap[3]));
                }
            } else {
                eprintln!("Unrecognized option --{}", &cap[1]);
                exit(1);
            }
        } else {
            if rom_path.is_none() {
                rom_path = Some(arg.clone());

                let regex = Regex::new(r"\.?[^./]*$").unwrap();
                base_path = Some(String::from(regex.replace(arg, "")));
            } else if ram_path.is_none() {
                ram_path = Some(arg.clone());
            } else {
                eprintln!("Unrecognized parameter {}", arg);
                exit(1);
            }
        }
    }

    if rom_path.is_none() {
        eprintln!(
"Usage: {} [Options...] <ROM> [RAM]

Options:
  --serial[=local-auto]
  --serial=server:<addr>
  --serial=<server addr>",
                  argv[0]);
        exit(1);
    }


    if ram_path.is_none() {
        ram_path = Some(format!("{}.sav", base_path.as_ref().unwrap()));
    }

    let mut addr_space = box AddressSpace::new(rom_path.as_ref().unwrap(),
                                               ram_path.as_ref().unwrap());
    let mut sys_params = rom::load_rom(addr_space.as_mut());
    sys_params.serial_conn_param = scp;

    let mut ui = UI::new(&sys_params.cartridge_name);

    let system_state = box SystemState::new(addr_space, sys_params, &mut ui);
    let mut system = box System::new(system_state, ui,
                                     base_path.take().unwrap());

    system.main_loop();
}
