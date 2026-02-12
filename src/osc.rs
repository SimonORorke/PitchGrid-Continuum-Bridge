use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
//use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use howlong::{Clock, SteadyClock, TimePoint};
use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};

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

    fn create_socket_addr(port: u16) -> SocketAddrV4 {
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)
        //SocketAddrV4::from_str(&format!("{}:{}", LOCAL_HOST, port)).unwrap()
    }

    pub fn start(&mut self,
                 tuning_received_callback: SharedTuningReceivedCallback,
                 connected_changed_callback: SharedConnectedChangedCallback) {
        println!("Osc.start");
        let is_connected = self.is_connected.clone();
        if is_connected.load(Ordering::SeqCst) {
            panic!("PitchGrid is already connected.");
        }
        let socket = UdpSocket::bind(Self::create_socket_addr(LISTENING_PORT)).unwrap();
        socket.connect(Self::create_socket_addr(SEND_TO_PORT)).unwrap();
        let socket_clone = socket.try_clone().unwrap();
        rayon::spawn(move || {
            Self::send_heartbeats(socket);
        });
        let last_ack_time = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::listen(socket_clone, is_connected, last_ack_time, tuning_received_callback);
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

    fn handle_tuning(args: Vec<OscType>, tuning_received_callback: SharedTuningReceivedCallback) {
        if let [
            OscType::Int(depth),
            OscType::Int(mode),
            OscType::Float(root_freq),
            OscType::Float(stretch),
            OscType::Float(skew),
            OscType::Int(mode_offset),
            OscType::Int(steps),
        ] = args[..] {
            tuning_received_callback(depth, mode, root_freq, stretch, skew, mode_offset, steps);
        } else {
            println!("Osc.handle_tuning Invalid tuning arguments.");
        }
    }

    fn listen(socket: UdpSocket, is_connected: Arc<AtomicBool>,
              last_ack_time: Arc<Mutex<TimePoint>>,
              tuning_received_callback: SharedTuningReceivedCallback) {
        println!("Osc.listen: starting");
        let mut buf = [0u8; decoder::MTU];
        loop {
            println!("Osc.listen: receiving packet from socket");
            match socket.recv(&mut buf) {
                Ok(size) => {
                    // println!("Received packet with size {} from: {}", size, addr);
                    let (_, packet) = decoder::decode_udp(&buf[..size]).unwrap();
                    match packet {
                        OscPacket::Message(msg) => {
                            println!("Osc.listen: message received:");
                            is_connected.store(true, Ordering::SeqCst);
                            *last_ack_time.lock().unwrap() = SteadyClock::now();
                            match msg.addr.as_str() {
                                HANDSHAKE_ACK_ADDR => {
                                    println!("    {HANDSHAKE_ACK_ADDR}");
                                }
                                TUNING_ADDR => {
                                    println!("    {TUNING_ADDR}");
                                    Self::handle_tuning(msg.args, tuning_received_callback.clone());
                                }
                                _ => {
                                    println!("    {:?}", msg);
                                }
                            }
                        }
                        OscPacket::Bundle(bundle) => {
                            println!("OSC Bundle: {:?}", bundle);
                        }
                    }
                }
                Err(e) => {
                    println!("Parse error receiving from socket: {}", e);
                    break;
                }
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

    fn send_heartbeats(socket: UdpSocket) {
        println!("Osc.send_heartbeats: starting");
        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: HANDSHAKE_ADDR.to_string(),
            args: vec![OscType::Int(1)],
        })).unwrap();
        loop {
            println!("Osc.send_heartbeats: sending heartbeat message");
            socket.send(&msg_buf).unwrap();
            println!("Osc.send_heartbeats: sent heartbeat message");
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
//const LOCAL_HOST: &str = "127.0.0.1";
const SEND_TO_PORT: u16 = 34562;
const TUNING_ADDR: &str = "/pitchgrid/plugin/tuning";
