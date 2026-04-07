use std::io::{ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};
use crate::tuning_params::TuningParams;

/// The socket addresses are as per the PitchGrid plugin docs:
///     Connection Details
///         Plugin listens on: Port 34562 (default, configurable)
///         Plugin sends to: Port 34561 (default, configurable)
///         Transport: UDP on localhost (127.0.0.1)
///         Heartbeat requirement: Client must send /pitchgrid/heartbeat at least once
///             every 2 seconds to maintain connection
///         Likewise, PitchGrid will send /pitchgrid/heartbeat/ack to acknowledge heartbeats
///         and therefore show that it's still going to send tunings.
pub struct Osc {
    /// Saves Controller.osc from having to be an Arc<Mutex>>.
    inner: Mutex<OscInner>,
    last_ack_time: Arc<Mutex<Option<Instant>>>,
}

impl Osc {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(OscInner { is_running: false, stopper_senders: vec![] }),
            last_ack_time: Arc::new(Mutex::new(None)),
        }
    }

    fn create_socket_addr(port: u16) -> SocketAddrV4 {
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)
    }

    pub fn start(&self, callbacks: Arc<dyn OscCallbacks>) {
        // println!("Osc.start");
        if self.is_pitchgrid_connected() {
            panic!("PitchGrid is already connected.");
        }
        let mut stopper_receivers: Vec<mpsc::Receiver<()>> = vec![];
        let mut inner = self.inner.lock().unwrap();
        for _ in 0..3 {
            let (stopper_sender, stopper_receiver) = mpsc::channel();
            stopper_receivers.push(stopper_receiver);
            inner.stopper_senders.push(stopper_sender);
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
        inner.is_running = true;
        drop(inner);

        rayon::spawn(move || {
            Self::send_heartbeats(send_socket, heartbeat_stopper);
        });
        let last_ack_time_clone = self.last_ack_time.clone();
        let callbacks_clone1 = callbacks.clone();
        rayon::spawn(move || {
            Self::listen(listen_socket, last_ack_time, callbacks_clone1, listen_stopper);
        });
        let callbacks_clone2 = callbacks.clone();
        rayon::spawn(move || {
            Self::monitor_connection(last_ack_time_clone, callbacks_clone2,
                                     monitor_stopper);
        });
    }

    pub fn stop(&self) {
        // println!("Osc.stop");
        IS_PITCHGRID_CONNECTED.store(false, Ordering::SeqCst);
        // Stop the threads.
        let mut inner = self.inner.lock().unwrap();
        for stopper_sender in inner.stopper_senders.drain(..) {
            stopper_sender.send(()).unwrap();
        }
        let last_ack_time_clone = self.last_ack_time.clone();
        *last_ack_time_clone.lock().unwrap() = None;
        inner.is_running = false;
        // println!("Osc.stop: stopped OSC");
    }

    pub fn is_pitchgrid_connected(&self) -> bool {
        IS_PITCHGRID_CONNECTED.load(Ordering::SeqCst)
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().unwrap().is_running
    }

    fn handle_tuning(args: Vec<OscType>, callbacks: Arc<dyn OscCallbacks>) {
        // println!("Osc.handle_tuning");
        if let [
            OscType::Int(depth),
            OscType::Int(mode),
            OscType::Float(root_freq),
            OscType::Float(stretch),
            OscType::Float(skew),
            OscType::Int(mode_offset),
            OscType::Int(steps),
            // ===================================================================================
            // pg34 Remove the condition-bindings above. Uncomment the condition-bindings below.
            // ===================================================================================
            // OscType::Int(mode),
            // OscType::Float(root_freq),
            // OscType::Float(stretch),
            // OscType::Float(skew),
            // OscType::Float(mode_offset),
            // OscType::Int(steps),
            // OscType::Int(mos_a),
            // OscType::Int(mos_b),
        ] = args[..] {
            rayon::spawn(move || {
                if !IS_PITCHGRID_CONNECTED.load(Ordering::SeqCst) {
                    // println!("Osc.handle_tuning: PitchGrid connected");
                    IS_PITCHGRID_CONNECTED.store(true, Ordering::SeqCst);
                    callbacks.on_osc_pitchgrid_connected_changed();
                }
                callbacks.on_osc_tuning_received(TuningParams::new(
                    depth, mode, root_freq, stretch, skew, mode_offset, steps));
                // ==============================================================================
                // pg34 Remove the function call above. Uncomment the function call below.
                // ==============================================================================
                // callbacks.on_osc_tuning_received(TuningParams::new(
                //     mode, root_freq, stretch, skew, mode_offset, steps, mos_a, mos_b,));
            });
        } else {
            // println!("Osc.handle_tuning Invalid tuning arguments.");
        }
    }

    // ==============================================================================
    // pg34 Update this listen function to handle the changed incoming messages.
    // Unless we need the spectrum and consonance messages, just acknowledge them
    // as valid messages and otherwise ignore them. That's what is already done below
    // for the heartbeat message.
    // ==============================================================================
    fn listen(
        socket: UdpSocket,
        last_ack_time: Arc<Mutex<Option<Instant>>>,
        callbacks: Arc<dyn OscCallbacks>,
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
                                // ===============================================================
                                // pg34 I understand that the address defined in HEARTBEAT_ACK_ADDR
                                // will no longer be used. So use HEARTBEAT_ADDR instead.
                                // ===============================================================
                                // The incoming heartbeat message is acknowledges as valid and
                                // otherwise ignored.
                                HEARTBEAT_ACK_ADDR => {
                                    // println!("    {HEARTBEAT_ACK_ADDR}");
                                }
                                TUNING_ADDR => {
                                    // println!("Osc.listen: Received {TUNING_ADDR}");
                                    // println!("    args: {:?}", msg.args);
                                    Self::handle_tuning(msg.args, callbacks.clone());
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
    fn monitor_connection(last_ack_time: Arc<Mutex<Option<Instant>>>,
                          callbacks: Arc<dyn OscCallbacks>,
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
                    callbacks.on_osc_pitchgrid_connected_changed();
                }
                if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(500)) {
                    // Sleep was interrupted
                    return;
                }
                // Slept for 500ms, proceeding
                continue;
            }
            let last_ack_time = maybe_last_ack_time.unwrap();
            let time_since_ack = current_time.duration_since(last_ack_time);
            let was_connected = IS_PITCHGRID_CONNECTED.load(Ordering::SeqCst);
            // println!("current_time = {:?}, last_ack_time = {:?}, time_since_ack = {:?}, was_connected = {}",
            //          current_time, last_ack_time, time_since_ack, was_connected );
            if time_since_ack > Duration::from_secs(2) { // No ack for 2 seconds
                // println!("Osc.monitor_connection: not connected");
                IS_PITCHGRID_CONNECTED.store(false, Ordering::SeqCst);
                if was_connected {
                    callbacks.on_osc_pitchgrid_connected_changed();
                }
            }
            // We don't want to notify connected here, as that will hav been done in real time
            // in handle_tuning.
            if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(500)) {
                // Sleep was interrupted
                return;
            }
            // Slept for 500ms, proceeding
        }
    }

    /// PitchGrid will send us messages if we send a heartbeat message at least every 2 seconds.
    /// So send PitchGrid a heartbeat message every second.
    // ===========================================================================================
    // pg34 Modify the heartbeat message sent to include the listening port,
    // which is defined by the LISTENING_PORT constant.
    // Later, I'll change it from a constant to a variable specified in the UI.
    // ===========================================================================================
    fn send_heartbeats(socket: UdpSocket, stopper_receiver: mpsc::Receiver<()>) {
        // println!("Osc.send_heartbeats: starting");
        let msg_buf = encoder::encode(&OscPacket::Message(OscMessage {
            addr: HEARTBEAT_ADDR.to_string(),
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

struct OscInner {
    is_running: bool,
    stopper_senders: Vec<mpsc::Sender<()>>,
}

// ===========================================================================================
// pg34 I understand that the address defined in HEARTBEAT_ACK_ADDR will no longer be used.
// So remove it and use HEARTBEAT_ADDR instead.
// ===========================================================================================
const HEARTBEAT_ACK_ADDR: &str = "/pitchgrid/heartbeat/ack";
const HEARTBEAT_ADDR: &str = "/pitchgrid/heartbeat";
const LISTENING_PORT: u16 = 34561;
const SEND_TO_PITCHGRID_PORT: u16 = 34562;
const TUNING_ADDR: &str = "/pitchgrid/plugin/tuning";

pub trait OscCallbacks: Send + Sync {
    fn on_osc_pitchgrid_connected_changed(&self);
    fn on_osc_tuning_received(&self, tuning_params: TuningParams);
}

static IS_PITCHGRID_CONNECTED: AtomicBool = AtomicBool::new(false);
