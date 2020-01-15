use hidapi::HidApi;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender, Receiver};

use crate::system_state::{UIEvent, UIScancode};


pub struct SC {
    events: Receiver<UIEvent>,
    rumble_on: Arc<AtomicBool>,
    rumble_off: Arc<AtomicBool>,

    #[allow(dead_code)]
    event_thread: std::thread::JoinHandle<()>,
}

struct SCThread {
    dev: hidapi::HidDevice,
    events: Sender<UIEvent>,
    rumble_on: Arc<AtomicBool>,
    rumble_off: Arc<AtomicBool>,
    rumble_state: bool,

    input_state: HashMap<UIScancode, bool>,
    button_map: HashMap<SCButton, UIScancode>,
}

#[derive(Serialize, Deserialize)]
struct SCInputData {
    always_one: u8,
    unknown_0: u8,

    status: u16,

    seqnum: u16,

    unknown_1: [u8; 2],

    buttons_0: u16,
    buttons_1: u8,

    lshoulder: u8,
    rshoulder: u8,

    unknown_2: [u8; 3],

    lpad_x: i16,
    lpad_y: i16,

    rpad_x: i16,
    rpad_y: i16,

    unknown_3: [u8; 4],

    acceleration_x: i16,
    acceleration_z: i16,
    acceleration_y: i16,

    rotation_x: i16,
    rotation_z: i16,
    rotation_y: i16,

    orientation_ya: i16,
    orientation_x: i16,
    orientation_z: i16,
    orientation_yb: i16,

    unknown_4: [u8; 16],

    #[serde(skip)]
    full_buttons: u32,
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum SCButton {
    BottomRShoulder = 0x00,
    BottomLShoulder = 0x01,
    TopRShoulder = 0x02,
    TopLShoulder = 0x03,

    Y = 0x04,
    B = 0x05,
    X = 0x06,
    A = 0x07,

    Up = 0x08,
    Right = 0x09,
    Left = 0x0a,
    Down = 0x0b,

    Previous = 0x0c,
    Action = 0x0d,
    Next = 0x0e,

    LGrip = 0x0f,
    RGrip = 0x10,

    LPad = 0x11,
    RPad = 0x12,

    LPadTouch = 0x13,
    RPadTouch = 0x14,

    AnalogStick = 0x15,

    VirtLeft = 0x16,
    VirtRight = 0x17,
    VirtUp = 0x18,
    VirtDown = 0x19,
}


impl SC {
    pub fn new() -> Option<Self> {
        let hidapi = match HidApi::new() {
            Ok(x) => x,
            Err(e) => {
                println!("Error initializing HIDAPI: {}", e);
                return None;
            },
        };

        let mut dev_found = None;
        for dev in hidapi.devices() {
            if dev.vendor_id == 0x28de &&
                ((dev.product_id == 0x1142 && dev.interface_number == 1) ||
                 (dev.product_id == 0x1102 && dev.interface_number == 2))
            {
                dev_found = Some(dev);
                break;
            }
        }

        let dev = match dev_found {
            Some(di) =>
                match di.open_device(&hidapi) {
                    Ok(x) => x,
                    Err(e) => {
                        println!("Error opening SC: {}", e);
                        return None;
                    },
                },

            None => return None,
        };

        let mut init_0 = [0u8; 65];
        init_0[1] = 0x83;

        dev.send_feature_report(&init_0).unwrap();

        let mut received = [0u8; 65];
        dev.get_feature_report(&mut received).unwrap();

        received[0] = 0x00;
        received[1] = 0xae;
        received[2] = 0x15;
        received[3] = 0x01;
        for i in 24..65 {
            received[i] = 0;
        }
        dev.send_feature_report(&received).unwrap();

        dev.get_feature_report(&mut received).unwrap();

        let mut init_1 = [0u8; 65];
        init_1[1] = 0x81;
        dev.send_feature_report(&init_1).unwrap();

        /* Configure input report */
        let cir: [u8; 65] = [
            /* report_id */
            0,
            /* command */
            0x87,
            /* ??? */
            0x15, 0x32, 0x84, 0x03,
            /* Some preprocessing for the touchpad values */
            0x08,
            /* ??? */
            0x00, 0x00, 0x31, 0x02, 0x00, 0x08, 0x07, 0x00,
            0x07, 0x07, 0x00, 0x30,
            /* input_mask: Acceleration, rotation, orientation */
            0x1c,
            /* ??? */
            0x2f, 0x01, 0x00,

            /* padding */
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        ];
        dev.send_feature_report(&cir).unwrap();


        let (events_s, events_r) = channel();
        let rumble_on = Arc::new(AtomicBool::new(false));
        let rumble_off = Arc::new(AtomicBool::new(false));
        let evt_thr = SCThread::spawn(dev, events_s,
                                      rumble_on.clone(), rumble_off.clone());

        Some(Self {
            events: events_r,
            rumble_on: rumble_on,
            rumble_off: rumble_off,
            event_thread: evt_thr,
        })
    }

    pub fn poll_event(&self) -> Option<UIEvent> {
        match self.events.try_recv() {
            Ok(x) => Some(x),
            Err(_) => None,
        }
    }

    pub fn rumble(&mut self, state: bool) {
        if state {
            self.rumble_on.store(true, Ordering::Relaxed);
        } else {
            self.rumble_off.store(true, Ordering::Relaxed);
        }
    }
}


impl SCThread {
    fn spawn(dev: hidapi::HidDevice, events: Sender<UIEvent>,
             rumble_on: Arc<AtomicBool>, rumble_off: Arc<AtomicBool>)
        -> std::thread::JoinHandle<()>
    {
        let mut is = HashMap::new();

        is.insert(UIScancode::CA, false);
        is.insert(UIScancode::CB, false);
        is.insert(UIScancode::CStart, false);
        is.insert(UIScancode::CSelect, false);
        is.insert(UIScancode::CLeft, false);
        is.insert(UIScancode::CRight, false);
        is.insert(UIScancode::CUp, false);
        is.insert(UIScancode::CDown, false);
        is.insert(UIScancode::CSkip, false);
        is.insert(UIScancode::CLoad(0), false);
        is.insert(UIScancode::CLoad(1), false);
        is.insert(UIScancode::CSave(0), false);
        is.insert(UIScancode::CSave(1), false);

        let mut bm = HashMap::new();

        bm.insert(SCButton::B, UIScancode::CA);
        bm.insert(SCButton::A, UIScancode::CB);
        bm.insert(SCButton::Y, UIScancode::CStart);
        bm.insert(SCButton::X, UIScancode::CSelect);
        bm.insert(SCButton::VirtLeft, UIScancode::CLeft);
        bm.insert(SCButton::VirtRight, UIScancode::CRight);
        bm.insert(SCButton::VirtUp, UIScancode::CUp);
        bm.insert(SCButton::VirtDown, UIScancode::CDown);
        bm.insert(SCButton::Next, UIScancode::CSkip);
        bm.insert(SCButton::BottomLShoulder, UIScancode::CLoad(0));
        bm.insert(SCButton::BottomRShoulder, UIScancode::CLoad(1));
        bm.insert(SCButton::LGrip, UIScancode::CSave(0));
        bm.insert(SCButton::RGrip, UIScancode::CSave(1));

        let obj = Self {
            dev: dev,
            events: events,
            rumble_on: rumble_on,
            rumble_off: rumble_off,
            rumble_state: false,
            input_state: is,
            button_map: bm,
        };

        std::thread::spawn(move || obj.update_loop())
    }

    fn send_rumble(&self, index: u8, intensity: u16, period: u16, count: u16) {
        let cmd: [u8; 65] = [
            0,
            0x8f, 0x67, index,
            (intensity & 0xff) as u8,
            (intensity >> 8) as u8,
            (period & 0xff) as u8,
            (period >> 8) as u8,
            (count & 0xff) as u8,
            (count >> 8) as u8,

            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0
        ];

        self.dev.send_feature_report(&cmd).unwrap();
    }

    fn update_loop(mut self) {
        let mut rbuf = [0u8; 64];

        loop {
            if self.rumble_off.load(Ordering::Relaxed) && self.rumble_state {
                self.rumble_state = false;
                self.rumble_off.store(false, Ordering::Relaxed);
            }

            if self.rumble_on.load(Ordering::Relaxed) {
                self.rumble_state = true;
                self.rumble_on.store(false, Ordering::Relaxed);
            }

            if self.rumble_state {
                self.send_rumble(0, 0xffff, 0, 1);
                self.send_rumble(1, 0xffff, 0, 1);
            }

            match self.dev.read_timeout(&mut rbuf, 1000) {
                Ok(x) => {
                    if x < 64 {
                        continue;
                    }
                },

                Err(e) => {
                    println!("Error while retrieving data from SC: {}", e);
                    return;
                }
            }

            let mut data = SCInputData::from_raw(&rbuf);
            if !data.is_valid() {
                continue;
            }

            data.construct_buttons();

            for (btn, sc) in &self.button_map {
                let state = data.button(*btn);
                if state != self.input_state.insert(*sc, state).unwrap() {
                    self.events.send(UIEvent::Key {
                        key: *sc,
                        down: state,
                    }).unwrap();
                }
            }
        }
    }
}


impl SCInputData {
    fn from_raw(raw_buf: &[u8; 64]) -> Self {
        bincode::deserialize(raw_buf).unwrap()
    }

    fn is_valid(&self) -> bool {
        self.status == 0x3c01
    }

    fn construct_buttons(&mut self) {
        self.full_buttons = self.buttons_0 as u32 |
                          ((self.buttons_1 as u32) << 16);

        if self.analog_valid() && self.button(SCButton::LPad) {
            self.full_buttons &= !(1u32 << (SCButton::LPad as usize));
            self.full_buttons |= 1u32 << (SCButton::AnalogStick as usize);
        }

        if self.lpad_x < -16384 {
            self.full_buttons |= 1u32 << (SCButton::VirtLeft as usize);
        }
        if self.lpad_x > 16384 {
            self.full_buttons |= 1u32 << (SCButton::VirtRight as usize);
        }
        if self.lpad_y > 16384 {
            self.full_buttons |= 1u32 << (SCButton::VirtUp as usize);
        }
        if self.lpad_y < -16384 {
            self.full_buttons |= 1u32 << (SCButton::VirtDown as usize);
        }
    }

    fn button(&self, btn: SCButton) -> bool {
        self.full_buttons & (1u32 << (btn as usize)) != 0
    }

    fn lpad_valid(&self) -> bool {
        self.button(SCButton::LPadTouch)
    }

    #[allow(dead_code)]
    fn rpad_valid(&self) -> bool {
        self.button(SCButton::RPadTouch)
    }

    fn analog_valid(&self) -> bool {
        !self.lpad_valid()
    }
}
