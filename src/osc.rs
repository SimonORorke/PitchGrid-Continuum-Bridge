use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use howlong::{Clock, SteadyClock, TimePoint};
use open_sound_control::*;

pub struct Osc {
    is_connected: Arc<AtomicBool>,
    last_ack_time: Arc<Mutex<TimePoint>>,
}

impl Osc {
    pub fn new() -> Self {
        Self {
            is_connected: Arc::new(AtomicBool::new(false)),
            last_ack_time: Arc::new(Mutex::new(SteadyClock::now())),
        }
    }

    pub fn start(&mut self,
                 tuning_received_callback: SharedTuningReceivedCallback,
                 connected_changed_callback: SharedConnectedChangedCallback) {
        println!("Osc.start");
        let is_connected = self.is_connected.clone();
        if is_connected.load(Ordering::SeqCst) {
            panic!("PitchGrid is already connected.");
        }
        rayon::spawn(move || {
            Self::send_heartbeats();
        });
        let last_ack_time = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::listen(is_connected, last_ack_time, tuning_received_callback);
        });
        let is_connected = self.is_connected.clone();
        let last_ack_time = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::monitor_connection(is_connected, last_ack_time, connected_changed_callback);
        });
    }

    pub fn stop(&mut self) {
        println!("Osc.stop");
        self.is_connected.store(false, Ordering::SeqCst);
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    fn handle_tuning(args: Vec<OscArgument>, tuning_received_callback: SharedTuningReceivedCallback) {
        if let [
            OscArgument::Int32(depth),
            OscArgument::Int32(mode),
            OscArgument::Float32(root_freq),
            OscArgument::Float32(stretch),
            OscArgument::Float32(skew),
            OscArgument::Int32(mode_offset),
            OscArgument::Int32(steps),
        ] = args[..] {
            // println!("Tuning: depth={}, mode={}, root={}Hz, stretch={}, skew={}, offset={}, steps={}",
            tuning_received_callback(depth, mode, root_freq, stretch, skew, mode_offset, steps);
        } else {
            panic!("Invalid tuning arguments.");
        }
    }

    fn listen(is_connected: Arc<AtomicBool>, last_ack_time: Arc<Mutex<TimePoint>>,
        tuning_received_callback: SharedTuningReceivedCallback) {
        println!("Osc.listen: starting");
        let receiver = OscReceiver::new(LISTENING_PORT.into()).unwrap();
        loop {
            match receiver.get_messages() {
                Ok(OscPacket::Message(msg)) => {
                    println!("Osc.listen: message received");
                    is_connected.store(true, Ordering::SeqCst);
                    *last_ack_time.lock().unwrap() = SteadyClock::now();
                    match msg.address.as_str() {
                        HANDSHAKE_ACK_ADDR => {},
                        TUNING_ADDR => {
                            Self::handle_tuning(msg.arguments, tuning_received_callback.clone());
                        },
                        _ => {
                            panic!("Invalid message address: {}", msg.address.as_str());
                        }
                    } 
                },
                Ok(OscPacket::Bundle(_bundle)) => panic!("Got bundle."),
                Err(err) => panic!("Parse error: {:?}", err),
            }
        }
    }

    fn monitor_connection(is_connected: Arc<AtomicBool>,
                          last_ack_time: Arc<Mutex<TimePoint>>,
                          connected_changed_callback: SharedConnectedChangedCallback) {
        println!("Osc.monitor_connection: starting");
        loop {
            let current_time = SteadyClock::now();
            let time_since_ack = current_time - *last_ack_time.lock().unwrap();
            let was_connected = is_connected.load(Ordering::SeqCst);
            if time_since_ack > Duration::from_secs(2) { // No ack for 2 seconds
                println!("Osc.monitor_connection: not connected");
                is_connected.store(false, Ordering::SeqCst);
                if was_connected {
                    connected_changed_callback();
                }
            } else if !was_connected { // Reconnected
                println!("Osc.monitor_connection: connected");
                is_connected.store(true, Ordering::SeqCst);
                connected_changed_callback();
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    fn send_heartbeats() {
        println!("Osc.send_heartbeats: starting");
        let message = OscMessage {
            address: String::from(HANDSHAKE_ADDR),
            arguments: vec![OscArgument::Int32(1)],
        };
        let sender = OscSender::new(LOCAL_HOST.to_string(), SEND_TO_PORT.into());
        loop {
            println!("Osc.send_heartbeats: sending heartbeat message:");
            sender.send_message(&message);
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

type SharedConnectedChangedCallback = Arc<dyn Fn() + Send + Sync + 'static>;

type SharedTuningReceivedCallback =
Arc<dyn Fn(
    i32, // depth
    i32, // mode
    f32, // root_freq
    f32, // stretch
    f32, // skew
    i32, // mode_offset
    i32 // steps
) + Send + Sync + 'static>;

const HANDSHAKE_ACK_ADDR: &str = "/pitchgrid/heartbeat/ack";
const HANDSHAKE_ADDR: &str = "/pitchgrid/heartbeat";
const LISTENING_PORT: u16 = 34561;
const LOCAL_HOST: &str = "127.0.0.1";
const SEND_TO_PORT: u16 = 34562;
const TUNING_ADDR: &str = "/pitchgrid/plugin/tuning";
