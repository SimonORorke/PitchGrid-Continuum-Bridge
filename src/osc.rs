use std::io::{ErrorKind, Write};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
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
    }

    pub fn start(&mut self,
                 tuning_received_callback: SharedTuningReceivedCallback,
                 connected_changed_callback: SharedConnectedChangedCallback) {
        println!("Osc.start");
        let is_connected = self.is_connected.clone();
        if is_connected.load(Ordering::SeqCst) {
            panic!("PitchGrid is already connected.");
        }
        // The socket addresses are as per the PitchGrid plugin docs:
        //     Connection Details
        //         Plugin listens on: Port 34562 (default, configurable)
        //         Plugin sends to: Port 34561 (default, configurable)
        //         Transport: UDP on localhost (127.0.0.1)
        //         Heartbeat requirement: Client must send /pitchgrid/heartbeat at least once
        //             every 2 seconds to maintain connection
        let listen_socket =
            UdpSocket::bind(Self::create_socket_addr(LISTENING_PORT)).unwrap();
        println!("Osc.start: listening on {}", listen_socket.local_addr().unwrap());
        std::io::stdout().flush().unwrap();
        let send_socket =
            UdpSocket::bind(Self::create_socket_addr(SEND_TO_PITCHGRID_PORT)).unwrap();
        println!("Osc.start: sending from {}", send_socket.local_addr().unwrap());
        std::io::stdout().flush().unwrap();
        let last_ack_time = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::send_heartbeats(send_socket);
        });
        let last_ack_time_clone = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::listen(listen_socket, is_connected, last_ack_time, tuning_received_callback);
        });
        let is_connected = self.is_connected.clone();
        rayon::spawn(move || {
            Self::monitor_connection(is_connected, last_ack_time_clone, connected_changed_callback);
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
        println!("Osc.handle_tuning");
        if let [
            OscType::Int(depth),
            OscType::Int(mode),
            OscType::Float(root_freq),
            OscType::Float(stretch),
            OscType::Float(skew),
            OscType::Int(mode_offset),
            OscType::Int(steps),
        ] = args[..] {
            rayon::spawn(move || {
                tuning_received_callback(depth, mode, root_freq, stretch, skew, mode_offset, steps);
            });
        } else {
            println!("Osc.handle_tuning Invalid tuning arguments.");
        }
    }

    fn listen(
        socket: UdpSocket,
        is_connected: Arc<AtomicBool>,
        last_ack_time: Arc<Mutex<TimePoint>>,
        tuning_received_callback: SharedTuningReceivedCallback) {
        socket.set_read_timeout(Some(Duration::from_millis(10))).unwrap();
        println!("Osc.listen: starting, listening on {}", socket.local_addr().unwrap());
        std::io::stdout().flush().unwrap();
        let mut buf = [0u8; decoder::MTU];
        loop {
            // println!("Osc.listen: receiving packet from socket");
            match socket.recv_from(&mut buf) {
                Ok((size, addr)) => {
                    println!("Osc.listen: received {} bytes from {}", size, addr);
                    std::io::stdout().flush().unwrap();
                    let decoded = decoder::decode_udp(&buf[..size]);
                    let (_, packet) = match decoded {
                        Ok(v) => v,
                        Err(err) => {
                            println!("OSC decode error: {}", err);
                            std::io::stdout().flush().unwrap();
                            continue;
                        }
                    };
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
                                    println!("    args: {:?}", msg.args);
                                    Self::handle_tuning(msg.args, tuning_received_callback.clone());
                                }
                                _ => {
                                    println!("    Unknown address: {}", msg.addr);  // Change this line
                                    // println!("    {:?}", msg);
                                }
                            }
                        }
                        OscPacket::Bundle(bundle) => {
                            println!("OSC Bundle: {:?}", bundle);
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        // Timeout - no data received, continue listening
                        continue;
                    }
                    // On Windows with a connected UDP socket, ICMP "Port Unreachable"
                    // shows up here as WSAECONNRESET (10054) / ErrorKind::ConnectionReset.
                    if e.kind() == ErrorKind::ConnectionReset {
                        println!("Osc.listen: Socket recv_from() got ConnectionReset (WSAECONNRESET/10054); ignoring and continuing");
                        break;
                    }
                    println!("Osc.listen: Socket error receiving from socket: {}", e);
                    break;
                }
            }
        }
    }

    /// Monitors the connection status of the socket.
    /// PitchGrid will send us messages if we send a heartbeat message at least every 2 seconds.
    /// So, if we don't receive any messages for 2 seconds, PitchGrid is probably not running.
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
                    let connected_changed_callback
                        = connected_changed_callback.clone();
                    rayon::spawn(move || {
                        connected_changed_callback();
                    });
                }
            } else if !was_connected { // Reconnected
                println!("Osc.monitor_connection: connected");
                is_connected.store(true, Ordering::SeqCst);
                let connected_changed_callback
                    = connected_changed_callback.clone();
                rayon::spawn(move || {
                    connected_changed_callback();
                });
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// PitchGrid will send us messages if we send a heartbeat message at least every 2 seconds.
    /// So send PitchGrid a heartbeat message every second.
    fn send_heartbeats(socket: UdpSocket) {
        println!("Osc.send_heartbeats: starting");
        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: HANDSHAKE_ADDR.to_string(),
            args: vec![OscType::Int(1)],
        })).unwrap();
        let socket_to_addr = Self::create_socket_addr(SEND_TO_PITCHGRID_PORT);
        loop {
            println!("Osc.send_heartbeats: sending heartbeat message to {}", socket_to_addr);
            match socket.send_to(&msg_buf, socket_to_addr) {
                Ok(bytes_sent) => {
                    println!("Osc.send_heartbeats: sent {} bytes", bytes_sent);
                    std::io::stdout().flush().unwrap();
                }
                Err(e) => {
                    println!("Osc.send_heartbeats: ERROR sending: {}", e);
                    std::io::stdout().flush().unwrap();
                }
            }
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
const SEND_TO_PITCHGRID_PORT: u16 = 34562;
const TUNING_ADDR: &str = "/pitchgrid/plugin/tuning";
