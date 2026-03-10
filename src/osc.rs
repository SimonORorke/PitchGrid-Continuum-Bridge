use std::io::{ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};

/// The socket addresses are as per the PitchGrid plugin docs:
///     Connection Details
///         Plugin listens on: Port 34562 (default, configurable)
///         Plugin sends to: Port 34561 (default, configurable)
///         Transport: UDP on localhost (127.0.0.1)
///         Heartbeat requirement: Client must send /pitchgrid/heartbeat at least once
///             every 2 seconds to maintain connection
pub struct Osc {
    is_connected: Arc<AtomicBool>,
    last_ack_time: Arc<Mutex<Option<Instant>>>,
    stopper_senders: Vec<mpsc::Sender<()>>,
}

impl Osc {
    pub fn new() -> Self {
        Self {
            is_connected: Arc::new(AtomicBool::new(false)),
            last_ack_time: Arc::new(Mutex::new(None)),
            stopper_senders: vec![],
        }
    }

    fn create_socket_addr(port: u16) -> SocketAddrV4 {
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)
    }

    pub fn start(&mut self,
                 tuning_received_callback: SharedTuningReceivedCallback,
                 connected_changed_callback: SharedConnectedChangedCallback) {
        // println!("Osc.start");
        let is_connected = self.is_connected.clone();
        if is_connected.load(Ordering::SeqCst) {
            panic!("PitchGrid is already connected.");
        }
        let mut stopper_receivers: Vec<mpsc::Receiver<()>> = vec![];
        for _ in 0..3 {
            let (stopper_sender, stopper_receiver) = mpsc::channel();
            stopper_receivers.push(stopper_receiver);
            self.stopper_senders.push(stopper_sender);
        }
        let mut stopper_receivers_iter = stopper_receivers.into_iter();
        let heartbeat_stopper = stopper_receivers_iter.next().unwrap();
        let listen_stopper = stopper_receivers_iter.next().unwrap();
        let monitor_stopper = stopper_receivers_iter.next().unwrap();

        let socket = UdpSocket::bind(Self::create_socket_addr(LISTENING_PORT)).unwrap();
        // println!("Osc.start: bound socket to {}", socket.local_addr().unwrap());
        let send_socket = socket.try_clone().unwrap();
        let listen_socket = socket;
        let last_ack_time = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::send_heartbeats(send_socket, heartbeat_stopper);
        });
        let last_ack_time_clone = self.last_ack_time.clone();
        rayon::spawn(move || {
            Self::listen(listen_socket, last_ack_time, tuning_received_callback, listen_stopper);
        });
        rayon::spawn(move || {
            Self::monitor_connection(is_connected, last_ack_time_clone, connected_changed_callback,
                                     monitor_stopper);
        });
    }

    pub fn stop(&mut self) {
        println!("Osc.stop: starting");
        self.is_connected.store(false, Ordering::SeqCst);
        // Stop the threads.
        for stopper_sender in self.stopper_senders.drain(..) {
            stopper_sender.send(()).unwrap();
        }
        let last_ack_time_clone = self.last_ack_time.clone();
        *last_ack_time_clone.lock().unwrap() = None;
        println!("Osc.stop: stopped OSC");
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    fn call_connected_changed_callback(
        connected_changed_callback: SharedConnectedChangedCallback) {
        connected_changed_callback();
    }

    fn handle_tuning(args: Vec<OscType>, tuning_received_callback: SharedTuningReceivedCallback) {
        // println!("Osc.handle_tuning");
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
        last_ack_time: Arc<Mutex<Option<Instant>>>,
        tuning_received_callback: SharedTuningReceivedCallback,
        stopper_receiver: mpsc::Receiver<()>) {
        socket.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        // println!("Osc.listen: starting, listening on {}", socket.local_addr().unwrap());
        let mut buf = [0u8; decoder::MTU];
        loop {
            // Check for stop signal
            if let Ok(_) = stopper_receiver.try_recv() {
                // Interrupted
                return;
            }
            // println!("Osc.listen: Waiting for packet...");
            match socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    // println!("Osc.listen: received {} bytes from {}", size, addr);
                    let decoded = decoder::decode_udp(&buf[..size]);
                    let (_, packet) = match decoded {
                        Ok(v) => v,
                        Err(err) => {
                            println!("OSC decode error: {}", err);
                            continue;
                        }
                    };
                    match packet {
                        OscPacket::Message(msg) => {
                            // println!("Osc.listen: message received:");
                            *last_ack_time.lock().unwrap() = Some(Instant::now());
                            match msg.addr.as_str() {
                                HANDSHAKE_ACK_ADDR => {
                                    // println!("    {HANDSHAKE_ACK_ADDR}");
                                }
                                TUNING_ADDR => {
                                    // println!("Osc.listen: Received {TUNING_ADDR}");
                                    // println!("    args: {:?}", msg.args);
                                    Self::handle_tuning(msg.args, tuning_received_callback.clone());
                                }
                                _ => {
                                    println!("Osc.listen: Received unknown address: {}", msg.addr);
                                }
                            }
                        }
                        OscPacket::Bundle(bundle) => {
                            println!("Osc.listen: Received OSC Bundle: {:?}", bundle);
                        }
                    }
                }
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        // Timeout - no data received, continue listening
                        continue;
                    }
                    if e.kind() == ErrorKind::TimedOut {
                        // Timeout - no data received, continue listening
                        continue;
                    }
                    // On Windows with a connected UDP socket, ICMP "Port Unreachable"
                    // shows up here as WSAECONNRESET (10054) / ErrorKind::ConnectionReset.
                    if e.kind() == ErrorKind::ConnectionReset {
                        // println!("Osc.listen: Socket recv_from() got ConnectionReset (WSAECONNRESET/10054); ignoring and continuing");
                        continue;
                    }
                    println!("Osc.listen: Socket error receiving from socket: {}", e);
                    break;
                }
            }
        }
    }

    /// Monitors the connection status of the socket.
    /// PitchGrid will send us messages if we send a heartbeat message at least every 2 seconds.
    /// So, if we don't receive any messages for 2 seconds, PitchGrid is probably not connected.
    fn monitor_connection(is_connected: Arc<AtomicBool>,
                          last_ack_time: Arc<Mutex<Option<Instant>>>,
                          connected_changed_callback: SharedConnectedChangedCallback,
                          stopper_receiver: mpsc::Receiver<()>) {
        // println!("Osc.monitor_connection: starting");
        let mut has_initially_not_connected_callback_been_called = false;
        loop {
            // println!("Osc.monitor_connection: looping");
            let current_time = Instant::now();
            let maybe_last_ack_time = *last_ack_time.lock().unwrap();
            if maybe_last_ack_time.is_none() {
                if !has_initially_not_connected_callback_been_called {
                    has_initially_not_connected_callback_been_called = true;
                    Self::call_connected_changed_callback(connected_changed_callback.clone());
                }
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            let last_ack_time = maybe_last_ack_time.unwrap();
            let time_since_ack = current_time.duration_since(last_ack_time);
            let was_connected = is_connected.load(Ordering::SeqCst);
            // println!("current_time = {:?}, last_ack_time = {:?}, time_since_ack = {:?}, was_connected = {}",
            //          current_time, last_ack_time, time_since_ack, was_connected );
            if time_since_ack > Duration::from_secs(2) { // No ack for 2 seconds
                // println!("Osc.monitor_connection: not connected");
                is_connected.store(false, Ordering::SeqCst);
                if was_connected {
                    Self::call_connected_changed_callback(connected_changed_callback.clone());
                }
            } else if time_since_ack <= Duration::from_secs(2)
                && !was_connected { // Reconnected
                // println!("Osc.monitor_connection: connected");
                is_connected.store(true, Ordering::SeqCst);
                Self::call_connected_changed_callback(connected_changed_callback.clone());
            }
            if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(500)) {
                // Sleep was interrupted
                return;
            }
            // Slept for 1s, proceeding
        }
    }

    /// PitchGrid will send us messages if we send a heartbeat message at least every 2 seconds.
    /// So send PitchGrid a heartbeat message every second.
    fn send_heartbeats(socket: UdpSocket, stopper_receiver: mpsc::Receiver<()>) {
        // println!("Osc.send_heartbeats: starting");
        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: HANDSHAKE_ADDR.to_string(),
            args: vec![OscType::Int(1)],
        })).unwrap();
        let socket_to_addr = Self::create_socket_addr(SEND_TO_PITCHGRID_PORT);
        loop {
            // println!("Osc.send_heartbeats: sending heartbeat message to {}", socket_to_addr);
            match socket.send_to(&msg_buf, socket_to_addr) {
                Ok(_bytes_sent) => {
                    // println!("Osc.send_heartbeats: sent {} bytes", bytes_sent);
                }
                Err(_e) => {
                    // println!("Osc.send_heartbeats: ERROR sending: {}", e);
                }
            }
            if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_secs(1)) {
                // Sleep was interrupted
                return;
            }
            // Slept for 1s, proceeding
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
