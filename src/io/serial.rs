use std::io::{Read, Write};
use std::sync::atomic::{AtomicU8, Ordering};

use crate::address_space::AddressSpace;
use crate::io::IOSpace;
use crate::io::int::Irq;
use crate::system_state::{IOReg, SystemState};
use crate::ui::UI;


const LINK_PORT: u16 = 0x9bc1u16; /* xgbc link */


pub enum SerialConnParam {
    Disabled,
    LocalAuto,
    #[cfg(target_os = "linux")]
    LocalSHM(usize),
    Client(String),
    Server(String),
}

impl SerialConnParam {
    pub fn default() -> Self {
        SerialConnParam::Disabled
    }
}


struct SerialSHM {
    remote_sb: &'static AtomicU8,
    remote_sc: &'static AtomicU8,
    remote_if: &'static AtomicU8,
}

pub struct SerialState {
    con: Option<std::net::TcpStream>,
    server: Option<std::net::TcpListener>,

    /* FIXME: Atomics */
    shm: Option<SerialSHM>,

    cycles_rem: Option<u32>,
}


impl SerialState {
    pub fn new(ui: &mut UI, param: &SerialConnParam) -> Option<Self> {
        let (addr, create_server, create_client) =
            match param {
                SerialConnParam::Disabled => return None,

                #[cfg(target_os = "linux")]
                SerialConnParam::LocalSHM(pid) => {
                    let shm_fd = AddressSpace::open_shm("hram", *pid);

                    let shm = AddressSpace::mmap(0, shm_fd, 0, 0x1000,
                                                 libc::PROT_READ |
                                                 libc::PROT_WRITE,
                                                 libc::MAP_SHARED, false);

                    let remote_sb = unsafe { AtomicU8::from_ptr((shm as *mut u8).offset(0xf01)) };
                    let remote_sc = unsafe { AtomicU8::from_ptr((shm as *mut u8).offset(0xf02)) };
                    let remote_if = unsafe { AtomicU8::from_ptr((shm as *mut u8).offset(0xf0f)) };

                    return Some(SerialState {
                        con: None,
                        server: None,

                        shm: Some(SerialSHM { remote_sb, remote_sc, remote_if }),

                        cycles_rem: None,
                    });
                },

                SerialConnParam::LocalAuto =>
                    (format!("localhost:{}", LINK_PORT), true, true),

                SerialConnParam::Client(addr) => (addr.clone(), false, true),
                SerialConnParam::Server(addr) => (addr.clone(), true, false),
            };

        let mut con = None;
        let mut server = None;

        if create_client {
            if let Ok(stream) = std::net::TcpStream::connect(&addr) {
                stream.set_nodelay(true).unwrap();
                stream.set_nonblocking(true).unwrap();
                con = Some(stream);
                ui.osd_message(format!("Connected to link server {}", addr));
            }
        }
        if create_server && con.is_none() {
            if let Ok(listener) = std::net::TcpListener::bind(&addr) {
                listener.set_nonblocking(true).unwrap();
                server = Some(listener);
            }
        }
        if con.is_none() && server.is_none() {
            match param {
                SerialConnParam::Disabled =>
                    unreachable!(),

                #[cfg(target_os = "linux")]
                SerialConnParam::LocalSHM(_) =>
                    unreachable!(),

                SerialConnParam::LocalAuto | SerialConnParam::Server(_) =>
                    ui.osd_message(String::from("Failed to set up link server")),

                SerialConnParam::Client(_) =>
                    ui.osd_message(String::from("Failed to connect to link server")),
            }

            return None;
        }

        Some(SerialState {
            con,
            server,

            shm: None,

            cycles_rem: None,
        })
    }

    pub fn vblank_check(&mut self) {
        if self.con.is_none() {
            if let Some(server) = self.server.as_mut() {
                if let Ok(con) = server.accept() {
                    /* TODO: Print this */
                    con.0.set_nodelay(true).unwrap();
                    con.0.set_nonblocking(true).unwrap();
                    self.con = Some(con.0);
                }
            }
        }
    }

    pub fn check_remote(&mut self, addr_space: &mut AddressSpace) {
        if addr_space.io_get_reg(IOReg::SC) & 0x81 == 0x80 {
            self.try_recv(addr_space);
        }
    }

    fn try_recv(&mut self, addr_space: &mut AddressSpace) {
        if let Some(con) = self.con.as_mut() {
            let mut recv_data = [0u8];

            let result = con.read(&mut recv_data);

            let recv_count =
                match result {
                    Ok(count) => Some(count),
                    Err(ref err) => {
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            Some(0)
                        } else {
                            None
                        }
                    },
                };

            if recv_count == Some(1) {
                let sc = addr_space.io_get_reg(IOReg::SC);

                if sc & 0x01 == 0 {
                    let send_data = [addr_space.io_get_reg(IOReg::SB)];
                    if con.write_all(&send_data).is_err() {
                        /* TODO: Print error somewhere */
                        self.conn_down();
                    }
                }

                addr_space.io_set_reg(IOReg::SB, recv_data[0]);
                addr_space.io_set_reg(IOReg::SC, sc & !0x80);

                let iflag = addr_space.io_get_reg(IOReg::IF);
                addr_space.io_set_reg(IOReg::IF, iflag | (Irq::Serial as u8));
            } else if recv_count.is_none() {
                /* TODO: Print this */
                self.conn_down();
            }
        } else if let Some(shm) = self.shm.as_mut() {
            let sc = addr_space.io_get_reg(IOReg::SC);

            if sc & 0x81 == 0x81 {
                let sb = addr_space.io_get_reg(IOReg::SB);

                let rsc = shm.remote_sc.load(Ordering::Relaxed);
                if rsc & 0x81 == 0x80 {
                    let rsb = shm.remote_sb.swap(sb, Ordering::Relaxed);
                    shm.remote_sc.store(rsc & 0x02, Ordering::Release);

                    shm.remote_if.fetch_or(Irq::Serial as u8,
                                           Ordering::AcqRel);

                    addr_space.io_set_reg(IOReg::SB, rsb);

                    println!("In: {:02x}; out: {:02x}", rsb, sb);
                } else {
                    addr_space.io_set_reg(IOReg::SB, 0);
                }

                addr_space.io_set_reg(IOReg::SC, sc & !0x80);

                let iflag = addr_space.io_get_reg(IOReg::IF);
                addr_space.io_set_reg(IOReg::IF,
                                      iflag | (Irq::Serial as u8));
            }
        }
    }

    fn conn_down(&mut self) {
        if let Some(con) = self.con.take() {
            con.shutdown(std::net::Shutdown::Both).unwrap_or(());
        }
    }

    pub fn add_cycles(&mut self, addr_space: &mut AddressSpace, dcycles: u32) {
        if let Some(cycles_rem) = self.cycles_rem {
            let (left, carry) = cycles_rem.overflowing_sub(dcycles);
            if carry {
                self.cycles_rem = None;
                self.try_recv(addr_space);
            } else {
                self.cycles_rem = Some(left);
            }
        }
    }
}


pub fn serial_write(sys_state: &mut SystemState, addr: u16, mut val: u8)
{
    match addr {
        0x01 => {
            sys_state.io_set_reg(IOReg::SB, val);
        },

        0x02 => {
            if !sys_state.cgb {
                val |= 0x02;
            }

            if let Some(serial) = sys_state.serial.as_mut() {
                serial.cycles_rem = None;
            }

            sys_state.io_set_reg(IOReg::SC, val & 0x83);

            if val & 0x80 != 0 {
                let sb = sys_state.io_get_reg(IOReg::SB);

                if let Some(serial) = sys_state.serial.as_mut() {
                    if let Some(con) = serial.con.as_mut() {
                        let mut recv_data = [0u8];
                        /* Drain remote */
                        while con.read(&mut recv_data).unwrap_or(0) == 1 {
                        }
                    }

                    if val & 0x01 != 0 {
                        if let Some(con) = serial.con.as_mut() {
                            let send_data = [sb];
                            if con.write_all(&send_data).is_err() {
                                serial.conn_down();
                            }
                        }

                        /* Takes 16 cycles of the shift clock
                         * (8 before start, then 8 to transfer) */
                        serial.cycles_rem = Some(
                            if sys_state.cgb && (val & 0x02 != 0) {
                                16 * 16
                            } else {
                                16 * 512
                            } - 1);
                    }
                }
            }
        }

        _ => {
            panic!("Unknown serial register 0xff{:02x} (w 0x{:02x})",
                   addr, val);
        }
    }
}
