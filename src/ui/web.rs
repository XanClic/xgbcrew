use std::cell::RefCell;
use std::collections::LinkedList;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext, CanvasRenderingContext2d,
              HtmlCanvasElement, ImageData, KeyboardEvent, MouseEvent};

use crate::ui::{AudioOutputParams, UIEvent, UIScancode};


pub struct WebBufferAudio {
    ctx: AudioContext,
    source: AudioBufferSourceNode,
    buffer: AudioBuffer,

    vblank_buf: Vec<f32>,
    vblank_single_channel_buf: Vec<f32>,
    cache_size: u32,

    time_ofs: f64,
    last_time: f64,
    sample_rate: f64,

    last_end: u32,
    buf_len: u32,
    channels: u32,
}

pub struct WebWorkletAudio {
    vblank_buf: Vec<f32>,
    vblank_single_channel_buf: Vec<f32>,
    cache_size: usize,

    buffer: Vec<f32>,
    ptrs: Vec<u32>,

    channels: usize,
}

pub struct WebUI {
    events: Rc<RefCell<LinkedList<UIEvent>>>,
    audio: Option<WebWorkletAudio>,
    video: CanvasRenderingContext2d,
    image_data: Option<ImageData>,
}


macro_rules! handle_single_key_event {
    ($events:expr, $object:expr, $event_name:literal, $down:literal) => {
        let events = $events.clone();
        let handler = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if let Some(evt) = Self::translate_key_event(&event, $down) {
                events.borrow_mut().push_back(evt);
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        $object.add_event_listener_with_callback($event_name, handler.as_ref().unchecked_ref()).unwrap();
        handler.forget();
    }
}

macro_rules! handle_single_button_event {
    ($events:expr, $window:expr, $element:literal, $event_name:literal, $scancode:expr, $down:literal) => {
        let events = $events.clone();
        let handler = Closure::wrap(Box::new(move |_: MouseEvent| {
            events.borrow_mut().push_back(UIEvent::Key { key: $scancode, down: $down });
        }) as Box<dyn FnMut(_)>);
        let a = $window.document().unwrap().get_element_by_id($element).unwrap();
        a.add_event_listener_with_callback($event_name, handler.as_ref().unchecked_ref()).unwrap();
        handler.forget();
    }
}

macro_rules! handle_key_event {
    ($events:expr, $object:expr) => {
        handle_single_key_event!($events, $object, "keydown", true);
        handle_single_key_event!($events, $object, "keyup", false);
    }
}

macro_rules! handle_button_event {
    ($events:expr, $window:expr, $element:literal, $scancode:expr) => {
        handle_single_button_event!($events, $window, $element, "pointerdown", $scancode, true);
        handle_single_button_event!($events, $window, $element, "pointerup", $scancode, false);
    }
}


impl WebUI {
    pub fn new() -> Self {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let canvas = document.get_element_by_id("fb").unwrap();
        let canvas: HtmlCanvasElement = canvas
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        let video_context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        let events: Rc<RefCell<LinkedList<UIEvent>>> = Default::default();

        handle_key_event!(events, window);

        handle_button_event!(events, window, "btn-a", UIScancode::X);
        handle_button_event!(events, window, "btn-b", UIScancode::Z);

        handle_button_event!(events, window, "btn-left", UIScancode::Left);
        handle_button_event!(events, window, "btn-right", UIScancode::Right);
        handle_button_event!(events, window, "btn-up", UIScancode::Up);
        handle_button_event!(events, window, "btn-down", UIScancode::Down);

        handle_button_event!(events, window, "btn-select", UIScancode::Backspace);
        handle_button_event!(events, window, "btn-start", UIScancode::Return);

        WebUI {
            events,
            audio: None,
            video: video_context,
            image_data: None,
        }
    }

    fn translate_key_event(event: &KeyboardEvent, down: bool) -> Option<UIEvent> {
        let ui_sc = match event.key().as_ref() {
            "p" | "P" => UIScancode::P,
            "x" | "X" => UIScancode::X,
            "z" | "Z" => UIScancode::Z,

            "Alt"       => UIScancode::Alt,
            "Control"   => UIScancode::Control,
            "Shift"     => UIScancode::Shift,

            " "         => UIScancode::Space,
            "Enter"     => UIScancode::Return,
            "Backspace" => UIScancode::Backspace,

            "ArrowLeft"     => UIScancode::Left,
            "ArrowRight"    => UIScancode::Right,
            "ArrowUp"       => UIScancode::Up,
            "ArrowDown"     => UIScancode::Down,

            "F1"    => UIScancode::F1,
            "F2"    => UIScancode::F2,
            "F3"    => UIScancode::F3,
            "F4"    => UIScancode::F4,
            "F5"    => UIScancode::F5,
            "F6"    => UIScancode::F6,
            "F7"    => UIScancode::F7,
            "F8"    => UIScancode::F8,
            "F9"    => UIScancode::F9,
            "F10"   => UIScancode::F10,
            "F11"   => UIScancode::F11,
            "F12"   => UIScancode::F12,

            _ => return None,
        };

        Some(UIEvent::Key { key: ui_sc, down })
    }

    pub fn setup_audio(&mut self, params: AudioOutputParams) {
        self.audio = WebWorkletAudio::new(params);
    }

    pub fn get_vblank_sound_buf(&mut self) -> Option<&mut Vec<f32>> {
        self.audio.as_mut()?.get_vblank_sound_buf()
    }

    pub fn submit_vblank_sound_buf(&mut self) {
        if let Some(audio) = self.audio.as_mut() {
            audio.submit_vblank_sound_buf();
        }
    }

    pub fn osd_drop_message(&mut self) {
    }

    pub fn osd_timed_message(&mut self, _text: String,
                             _duration: std::time::Duration)
    {
    }

    pub fn present_frame(&mut self, pixels: &[u32; 160 * 144]) {
        // oh no extremely unsafe
        if self.image_data.is_none() {
            let c = wasm_bindgen::Clamped(unsafe {
                std::slice::from_raw_parts(pixels as *const u32 as *const u8, 160 * 144 * 4)
            });
            let id = ImageData::new_with_u8_clamped_array_and_sh(c, 160, 144).unwrap();
            self.image_data = Some(id);
        }

        self.video.put_image_data(self.image_data.as_ref().unwrap(), 0.0, 0.0).unwrap();
    }

    pub fn poll_event(&mut self) -> Option<UIEvent> {
        self.events.borrow_mut().pop_front()
    }

    pub fn wait_event(&mut self, _timeout: std::time::Duration)
        -> Option<UIEvent>
    {
        // FIXME
        self.poll_event()
    }

    pub fn enable_sgb_border(&mut self) {
    }

    pub fn set_sgb_border(&mut self, _pixels: &[u32; 256 * 224]) {
    }

    pub fn set_fullscreen(&mut self, _state: bool) {
    }

    pub fn set_paused(&mut self, _paused: bool) {
    }

    pub fn get_sound_ringbuf(&self) -> Option<&[f32]> {
        self.audio.as_ref().map(|a| a.get_sound_ringbuf())
    }

    pub fn get_sound_ringbuf_ptrs(&mut self) -> Option<&mut [u32]> {
        self.audio.as_mut().map(|a| a.get_sound_ringbuf_ptrs())
    }
}

impl WebBufferAudio {
    fn new(params: AudioOutputParams) -> Option<Self> {
        let ctx = AudioContext::new().ok()?;
        let source = ctx.create_buffer_source().ok()?;
        let buf_len = (params.freq as u32) * 1;
        // FIXME: Support params.channels
        let buffer = ctx.create_buffer(1, buf_len, params.freq as f32).ok()?;

        source.set_loop(true);
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&ctx.destination()).ok()?;

        let time_ofs = ctx.current_time();
        source.start().ok()?;

        Some(WebBufferAudio {
            ctx,
            source,
            buffer,

            vblank_buf: Default::default(),
            vblank_single_channel_buf: Default::default(),
            cache_size: 44100 / 30, // 1/30th of a second

            time_ofs,
            last_time: time_ofs,
            sample_rate: params.freq as f64,

            last_end: 0,
            buf_len,
            channels: params.channels as u32,
        })
    }

    fn get_vblank_sound_buf(&mut self) -> Option<&mut Vec<f32>> {
        let now = self.ctx.current_time();
        let cur_pos = (((now - self.time_ofs) * self.sample_rate) as u32) % self.buf_len;

        let min_cache_size = ((now - self.last_time) * self.sample_rate) as u32;
        let cached = if self.cache_size < min_cache_size + 44100 / 60 {
            self.cache_size = min_cache_size + 44100 / 60;
            0
        } else {
            (self.last_end + self.buf_len - cur_pos) % self.buf_len
        };

        self.last_time = now;

        if cached >= self.cache_size {
            return None;
        }

        if self.cache_size >= self.buf_len / 2 {
            return None;
        }

        let sz = (self.cache_size - cached) * self.channels;
        self.vblank_buf.resize(sz as usize, 0.0);
        Some(&mut self.vblank_buf)
    }

    fn submit_vblank_sound_buf(&mut self) {
        let samples = self.vblank_buf.len() as u32 / self.channels;
        if self.vblank_single_channel_buf.len() < samples as usize {
            self.vblank_single_channel_buf.resize(samples as usize, 0.0);
        }
        for i in 0..samples {
            self.vblank_single_channel_buf[i as usize] = self.vblank_buf[(i * self.channels) as usize];
        }

        if self.last_end + samples <= self.buf_len {
            self.buffer.copy_to_channel_with_start_in_channel(self.vblank_single_channel_buf.as_slice(), 0, self.last_end).unwrap();
        } else {
            let head = (self.buf_len - self.last_end) as usize;
            let (head, tail) = self.vblank_single_channel_buf.split_at(head);
            self.buffer.copy_to_channel_with_start_in_channel(head, 0, self.last_end).unwrap();
            self.buffer.copy_to_channel_with_start_in_channel(tail, 0, 0).unwrap();
        }
        self.last_end = (self.last_end + samples) % self.buf_len;
    }
}

impl WebWorkletAudio {
    fn new(params: AudioOutputParams) -> Option<Self> {
        Some(WebWorkletAudio {
            vblank_buf: Default::default(),
            vblank_single_channel_buf: Default::default(),
            cache_size: 44100 / 30, // 1/30th of a second

            buffer: vec![0.0; 44100],
            ptrs: vec![0; 2],

            // FIXME: DO NOT IGNORE params.freq
            channels: params.channels,
        })
    }

    fn get_sound_ringbuf(&self) -> &[f32] {
        self.buffer.as_slice()
    }

    fn get_sound_ringbuf_ptrs(&mut self) -> &mut [u32] {
        self.ptrs.as_mut_slice()
    }

    fn get_vblank_sound_buf(&mut self) -> Option<&mut Vec<f32>> {
        let cached = (self.ptrs[0] as usize + self.buffer.len() - self.ptrs[1] as usize) % self.buffer.len();

        if cached >= self.cache_size {
            return None;
        }

        let sz = (self.cache_size - cached) * self.channels;
        self.vblank_buf.resize(sz, 0.0);
        Some(&mut self.vblank_buf)
    }

    fn submit_vblank_sound_buf(&mut self) {
        let samples = self.vblank_buf.len() / self.channels;
        if self.vblank_single_channel_buf.len() < samples {
            self.vblank_single_channel_buf.resize(samples, 0.0);
        }
        for i in 0..samples {
            self.vblank_single_channel_buf[i] = self.vblank_buf[(i * self.channels)];
        }

        let ipos = self.ptrs[0] as usize;
        if ipos + samples <= self.buffer.len() {
            self.buffer[ipos..(ipos + samples)].copy_from_slice(&self.vblank_single_channel_buf[..samples]);
        } else {
            let head = self.buffer.len() - ipos;
            let (head, tail) = self.vblank_single_channel_buf.split_at(head);
            let tail_len = samples - head.len();

            self.buffer[ipos..].copy_from_slice(head);
            self.buffer[..tail_len].copy_from_slice(&tail[..tail_len])
        }
        self.ptrs[0] = ((ipos + samples) % self.buffer.len()) as u32;
    }
}
