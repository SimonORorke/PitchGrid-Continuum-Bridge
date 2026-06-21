use crate::i_midi_manager::{IMidiManager, MidiInputListener, SharedOutput};
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort, MidiOutput, MidiOutputPort,
};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread::sleep;
use crate::global::{DeviceType};
use crate::midi_ports::{Io, IIo};
use crate::device_strategy::DeviceStrategy;
use log::trace;

/// The generic MIDI reception state: whether data is arriving and when the last message arrived.
/// Held behind an `Arc` so the input callback and the monitor / watchdog threads can each own a
/// clone — a `'static + Send` closure cannot borrow `&self`. The Continuum-specific state now lives
/// in `ContinuumProtocol`.
struct MidiInputState {
    is_receiving_data: AtomicBool,
    last_message_received_time: Mutex<Option<Instant>>,
}

impl MidiInputState {
    fn new() -> Self {
        Self {
            is_receiving_data: AtomicBool::new(false),
            last_message_received_time: Mutex::new(None),
        }
    }

    /// Records that a message has just arrived. Returns whether it is the first message since
    /// monitoring started, so the caller can raise `on_receiving_data_started`.
    fn record_message(&self) -> bool {
        trace!("record_message");
        self.is_receiving_data.store(true, Ordering::Relaxed);
        let mut last_time = self.last_message_received_time.lock().unwrap();
        let is_first_message = last_time.is_none();
        *last_time = Some(Instant::now());
        is_first_message
    }
}

/// A manager for MIDI devices and messages. It owns the device connections and the generic
/// reception state, and raises raw messages and connection-lifecycle events to its
/// `MidiInputListener` (the `ContinuumProtocol`). It knows nothing of the Continuum protocol itself.
pub struct MidiManager {
    connection_monitor_stopper_sender: Option<mpsc::Sender<()>>,
    input: Io<MidiInputPort>,
    input_connection: Option<MidiInputConnection<()>>,
    is_connection_monitor_running: bool,
    output: Io<MidiOutputPort>,
    /// The output connection, shared with the `MidiSender` that writes to it. `MidiManager` connects
    /// and disconnects it; the sender reads it. Replaces the former `OUTPUT_CONNECTION` global.
    output_connection: SharedOutput,
    /// The generic reception state, shared with the input callback and the spawned monitor /
    /// watchdog threads. Replaces the reception fields of the former `midi_refs` statics.
    input_state: Arc<MidiInputState>,
    /// The interpreter of inbound MIDI (the `ContinuumProtocol`), injected by `Presenter::new`.
    /// `MidiManager` raises raw events to it synchronously; it does any thread hand-off before
    /// re-entering the `Presenter`.
    listener: Arc<dyn MidiInputListener>,
}

/// For public self methods, see `impl IMidiManager for MidiManager`.
impl MidiManager {
    pub fn new(output_connection: SharedOutput, listener: Arc<dyn MidiInputListener>) -> Self {
        Self {
            connection_monitor_stopper_sender: None,
            input: Io::<MidiInputPort>::new(Box::new(Self::create_midi_input())),
            input_connection: None,
            is_connection_monitor_running: false,
            output: Io::<MidiOutputPort>::new(Box::new(Self::create_midi_output())),
            output_connection,
            input_state: Arc::new(MidiInputState::new()),
            listener,
        }
    }

    fn connect_input_device(
        &mut self,
        index: usize,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>> {
        trace!("connect_input_device: start");
        self.disconnect_input_device();
        // The input callback runs on midir's own thread and must be `'static + Send`, so it owns
        // clones of the shared reception state and the listener rather than borrowing `&self`.
        let input_state = Arc::clone(&self.input_state);
        let listener = Arc::clone(&self.listener);
        let input = &mut self.input;
        if let Some(port) = input.ports().get(index) {
            trace!("connect_input_device: found port");
            let device_name = port.device_name();
            let midi_port = port.midi_port();
            let midi_input = Self::create_midi_input();
            match midi_input.connect(
                midi_port,
                &device_name,
                move |_, message, _| {
                    if input_state.record_message() {
                        listener.on_receiving_data_started();
                    }
                    listener.on_message(message);
                },
                (),
            ) {
                Ok(connection) => {
                    input.set_port(port.clone());
                    let connection_option = Option::from(connection);
                    self.input_connection = connection_option;
                    trace!("connect_input_device: success");
                }
                Err(_) => {
                    trace!("connect_input_device: error");
                    // See comment in connect_output_device.
                    return Err(device_strategy.msg_cannot_connect(&device_name).into());
                }
            }
        }
        // If we have not yet received any messages from the instrument after 2 seconds,
        // show a message to the user.
        // Is this redundant?
        let watchdog_state = Arc::clone(&self.input_state);
        let watchdog_listener = Arc::clone(&self.listener);
        rayon::spawn(move || {
            sleep(Duration::from_secs(MIDI_WAIT_SECS));
            if !watchdog_state.is_receiving_data.load(Ordering::Relaxed) {
                trace!("connect_input_device: Stopped receiving data");
                watchdog_listener.on_receiving_data_stopped();
            }
        });
        Ok(())
    }

    fn connect_output_device(
        &mut self,
        index: usize,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>> {
        self.disconnect_output_device();
        let output = &mut self.output;
        if let Some(port) = output.ports().get(index) {
            let device_name = port.device_name();
            let midi_port = port.midi_port();
            let midi_output = Self::create_midi_output();
            match midi_output.connect(midi_port, &device_name) {
                Ok(connection) => {
                    *self.output_connection.lock().unwrap_or_else(|e| e.into_inner()) = Option::from(connection);
                    output.set_port(port.clone());
                }
                Err(_) =>
                // Devices that have their own MIDI drivers may support shared connections.
                // iConnectivity devices do.
                // On 7th Feb 2026, I asked in the iConnectivity User Community FB group,
                // in a post headed 'Exclusive lock on MIDI ports?',
                // whether an iConnectivity might in future support exclusive connections,
                // which would be useful for this application. There was no response.
                // So on 14th Feb 2026, I raised a support ticket for the feature request.
                // So far, no response.
                //
                // Also, the new Windows MIDI Services supports shared connections
                // ("multi-client") by default.
                // I don't know about other operating systems.
                // As of 4th Feb 2026, I now have Windows MIDI Services on my PC.
                // I don't see how to disable multi-client support.
                // So I currently cannot test exclusive connections any more.
                {
                    return Err(device_strategy.msg_cannot_connect(&device_name).into());
                }
            }
        }
        Ok(())
    }

    fn create_midi_input() -> MidiInput {
        MidiInput::new(INPUT_CLIENT_NAME).unwrap()
    }

    fn create_midi_output() -> MidiOutput {
        MidiOutput::new(OUTPUT_CLIENT_NAME).unwrap()
    }

    fn disconnect_input_device(&mut self) {
        trace!("disconnect_input_device start");
        let input_connection = self.input_connection.take();
        if let Some(connection) = input_connection {
            connection.close();
            let input = &mut self.input;
            input.set_port_to_none();
        }
    }

    fn disconnect_output_device(&mut self) {
        trace!("disconnect_output_device start");
        let connection_opt = self.output_connection.lock().unwrap().take();
        if let Some(connection) = connection_opt {
            connection.close();
            let output = &mut self.output;
            output.set_port_to_none();
        }
    }

    /// Monitor the connection status of the instrument.
    /// When the instrument has nothing else to send, it will send a sequence of heartbeat messages
    /// once a second and the editor will send back a sequence of heartbeat messages less than half
    /// a second later. This application will receive both sets of heartbeat messages. To be safe,
    /// if we have not had any data for 2 seconds, we assume
    /// the editor or instrument has disconnected.
    fn monitor_instrument_connection(
        input_state: Arc<MidiInputState>,
        listener: Arc<dyn MidiInputListener>,
        stopper_receiver: mpsc::Receiver<()>,
    ) {
        let start_time = Instant::now();
        let mut has_initially_not_connected_callback_been_called = false;
        loop {
            if input_state.is_receiving_data.load(Ordering::Relaxed) {
                let now = Instant::now();
                let last_time =
                    *input_state.last_message_received_time.lock().unwrap();
                if let Some(last_time) = last_time {
                    let duration = now.duration_since(last_time);
                    let seconds = duration.as_secs();
                    if seconds > MIDI_WAIT_SECS {
                        trace!("monitor_instrument_connection: Instrument disconnected.");
                        *input_state.last_message_received_time.lock().unwrap() = None;
                        input_state.is_receiving_data.store(false, Ordering::Relaxed);
                        listener.on_receiving_data_stopped();
                    }
                }
            } else if !has_initially_not_connected_callback_been_called {
                let now = Instant::now();
                let duration = now.duration_since(start_time);
                let seconds = duration.as_secs();
                // Give a chance for the instrument heartbeat messages to arrive.
                if seconds > MIDI_WAIT_SECS {
                    trace!("monitor_instrument_connection: Instrument not connected for 2 seconds on startup.");
                    // Not connected for 2 seconds after application start.
                    // So we can assume that the instrument is not yet connected.
                    // Provide an opportunity for a helpful message to be displayed.
                    listener.on_receiving_data_stopped();
                    has_initially_not_connected_callback_been_called = true;
                }
            }
            if stopper_receiver.recv_timeout(Duration::from_millis(500)).is_ok() {
                // Sleep was interrupted
                return;
            }
            // Slept for 500ms, proceeding
        }
    }

    fn refresh_input_devices(&mut self, input_device_name: &str) {
        trace!("refresh_input_devices: start");
        self.disconnect_input_device();
        self.input.populate_devices(input_device_name);
    }

    fn refresh_output_devices(&mut self, output_device_name: &str) {
        trace!("refresh_output_devices: start");
        self.disconnect_output_device();
        self.output.populate_devices(output_device_name);
    }

}

impl IMidiManager for MidiManager {
    /// Return whether both input and output devices are connected.
    fn are_devices_connected(&self) -> bool {
        if self.input_connection.is_none() {
            return false;
        }
        self.is_output_device_connected()
    }

    fn close(&mut self) {
        trace!("close");
        self.disconnect_input_device();
        self.disconnect_output_device();
        // self.stop_download_monitor();
        self.stop_instrument_connection_monitor();
    }

    fn connect_device(
        &mut self,
        index: usize,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>> {
        let were_ports_connected = self.are_devices_connected();
        // self.stop_download_monitor();
        self.stop_instrument_connection_monitor();
        match device_strategy.device_type() {
            DeviceType::Input => self.connect_input_device(index, device_strategy)?,
            DeviceType::Output => self.connect_output_device(index, device_strategy)?,
        }
        if !were_ports_connected {
            // The other port was already connected, so now they both are.
            if self.are_devices_connected() {
                trace!("connect_device {:?}: Calling on_devices_connected_changed \
                    because both ports are now connected", device_strategy.device_type());
                // Raised synchronously; the listener hands off to the Presenter itself.
                self.listener.on_devices_connected_changed();
            }
        }
        Ok(())
    }

    fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
    ) {
        self.input.populate_devices(input_device_name);
        self.output.populate_devices(output_device_name);
    }

    fn input(&self) -> &dyn IIo {
        &self.input
    }

    fn io(&self, device_strategy: &dyn DeviceStrategy) -> &dyn IIo {
        device_strategy.io(self)
    }

    fn is_output_device_connected(&self) -> bool {
        self.output_connection.lock().unwrap().is_some()
    }

    /// We should receive data from the instrument at least once per second, as it sends heartbeat
    /// messages at 1-second intervals when not otherwise busy.
    /// So, we can use this method to check if the instrument is still connected.
    fn is_receiving_data(&self) -> bool {
        self.input_state.is_receiving_data.load(Ordering::Relaxed)
    }

    fn output(&self) -> &dyn IIo {
        &self.output
    }

    fn refresh_devices(
        &mut self,
        device_name: &str,
        device_strategy: &dyn DeviceStrategy,
    ) {
        let were_devices_connected = self.are_devices_connected();
        // self.stop_download_monitor();
        self.stop_instrument_connection_monitor();
        match device_strategy.device_type() {
            DeviceType::Input => self.refresh_input_devices(device_name),
            DeviceType::Output => self.refresh_output_devices(device_name),
        }
        if were_devices_connected {
            // We have just disconnected one of the ports.
            trace!("refresh_devices: Calling on_devices_connected_changed because we have just disconnected one of the ports");
            self.listener.on_devices_connected_changed();
        }
    }

    fn start_instrument_connection_monitor(&mut self) {
        trace!("start_instrument_connection_monitor");
        *self.input_state.last_message_received_time.lock().unwrap() = None;
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.connection_monitor_stopper_sender = Some(stopper_sender);
        let input_state = Arc::clone(&self.input_state);
        let listener = Arc::clone(&self.listener);
        rayon::spawn(move || {
            Self::monitor_instrument_connection(input_state, listener, stopper_receiver);
        });
        self.is_connection_monitor_running = true;
    }

    fn stop_instrument_connection_monitor(&mut self) {
        trace!("stop_instrument_connection_monitor");
        if !self.is_connection_monitor_running {
            trace!("stop_instrument_connection_monitor: Already stopped.");
            return;
        }
        let stopper_sender =
            self.connection_monitor_stopper_sender.take();
        if stopper_sender.is_none() { return; }
        stopper_sender.unwrap().send(()).unwrap_or_else(|_| {
            panic!("stop_instrument_connection_monitor: Failed to send stop signal to connection monitor");
        });
        trace!("stop_instrument_connection_monitor: Stopped monitor thread.");
        self.is_connection_monitor_running = false;
        self.input_state.is_receiving_data.store(false, Ordering::Relaxed);
        trace!("stop_instrument_connection_monitor: Done.");
    }
}

const INPUT_CLIENT_NAME: &str = "My MIDI Input";
const MIDI_WAIT_SECS: u64 = 2;
const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";
