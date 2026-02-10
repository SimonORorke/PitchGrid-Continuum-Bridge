use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use open_sound_control::*;

const HANDSHAKE_ACK_ADDR: &str = "/pitchgrid/heartbeat/ack";
const HANDSHAKE_ADDR: &str = "/pitchgrid/heartbeat";
const LOCAL_HOST: &str = "127.0.0.1";
const RECEIVER_PORT: u16 = 34562;
const SENDER_PORT: u16 = 34561;
const TUNING_ADDR: &str = "/pitchgrid/plugin/tuning";

pub struct Osc {
    is_connected: Arc<AtomicBool>,
    receiver: Option<OscReceiver>,
    tuning_callback: Box<dyn Fn(
        i32, // depth
        i32, // mode
        f32, // root_freq
        i32, // stretch
        f32, // skew
        i32, // mode_offset
        i32)>, // steps
}

impl Osc {
    pub fn new(tuning_callback: Box<dyn Fn(i32, i32, f32, i32, f32, i32, i32)>) -> Self {
        Self {
            is_connected: Arc::new(AtomicBool::new(false)),
            receiver: None,
            tuning_callback
        }
    }

    pub fn start(&mut self) {
        self.receiver = Option::from(OscReceiver::new(RECEIVER_PORT.into()).unwrap());
        let is_connected = self.is_connected.clone();
        if is_connected.load(Ordering::SeqCst) {
            panic!("PitchGrid is already connected.");
        }
        rayon::spawn(move || {
            Self::send_heartbeats();
        });
    }

    fn send_heartbeats() {
        loop {
            let sender = OscSender::new(LOCAL_HOST.to_string(), SENDER_PORT.into());
            sender.send_message(&OscMessage {
                address: String::from(HANDSHAKE_ADDR),
                arguments: vec![OscArgument::Int32(1), ]
            });
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}