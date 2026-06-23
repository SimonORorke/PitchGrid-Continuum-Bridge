use std::error::Error;
use std::sync::{Arc, Mutex, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use log::{debug, trace};
use crate::i_midi_manager::{IMidiManager, SharedMidiManager, SharedOutput};
use crate::i_osc::{IOsc, OscCallbacks};
use crate::osc::Osc;
use crate::i_settings::ISettings;
use crate::device_strategy::{
    InputStrategy, OutputStrategy, DeviceStrategy};
use crate::settings::Settings;
use crate::midi_manager::MidiManager;
use crate::continuum_protocol::ContinuumProtocol;
use crate::i_continuum_protocol::{ContinuumProtocolListener, IContinuumProtocol};
use crate::midi_sender::{IMidiSender, MidiSender, NullMidiSender, SharedMidiSender};
use crate::i_ui_methods::IUiMethods;
use crate::presentation::Presentation;
use crate::i_tuner::SharedTuner;
use crate::tuner::Tuner;
use crate::tuning_params::TuningParams;
use crate::tuning_update_watchdog::TuningUpdateWatchdog;

/// This type plays the **Presenter** in the Model-View-Presenter (MVP) pattern — specifically the
/// Passive View / Humble Object variant: `IUiMethods` reduces the View to dumb setters, and the
/// Presenter pushes formatted state into that view abstraction (the view never pulls from a model).
/// (The code was originally described as MVC, but is really MVP; the type was renamed from
/// `Controller` to `Presenter` to reflect that.)
///
/// In this application, `Presenter` is also a coordinator between the various model components and
/// the view. This is useful for two interrelated reasons.
/// * Processes in the model components depend on the events of other model components to start.
/// * So many things external to the application can go wrong that event progress must be displayed
///   to facilitate user action when required.
///
/// `Presentation` provides the model with a conduit for sending messages to the view to be shown:
/// see `TuningUpdateWatchdog` for the only current example. It allows the model not to need to
/// know about view or the `Preseter` itself. In that respect, I'd say it conforms with MVP:
/// messaging facilities in MVVM are similar.
///
/// `DeviceStrategy` contains both view and presenter methods: that seems acceptable, as it is an
/// implementation of the strategy pattern.
///
/// The Slint UI, main.rs and `UiMethods` are the remainder of the view.
///
/// Everything else is the model.
///
pub struct Presenter {
    /// The view-facing facade. Owns the injected `IUiMethods` and every user-facing message
    /// string; the Presenter pushes formatted state through its intention-named methods.
    presentation: Presentation,
    /// Watches for the instrument's confirmation that a tuning update was applied and reports a
    /// timeout to the view if none arrives. Owns the former `await_tuning_updated_*` /
    /// `is_awaiting_*` state.
    tuning_update_watchdog: TuningUpdateWatchdog,
    /// True while the in-flight tuning send was triggered by a new instrument preset being
    /// selected (rather than a fresh tuning), so on_tuning_updated can show a tailored
    /// confirmation. An AtomicBool because it is set from the `&self` callback methods.
    is_preset_reselect: AtomicBool,
    osc: Box<dyn IOsc>,
    settings: Box<dyn ISettings>,
    tuner: SharedTuner,
    /// The MIDI manager, injected (like osc/settings/tuner) rather than reached through a global
    /// singleton. Shared because callback methods clone the `Arc` to pass it around.
    midi_manager: SharedMidiManager,
    midi_sender: SharedMidiSender,
    /// The Continuum-protocol interpreter, injected. The Presenter queries it for download state;
    /// the same instance is the MidiManager's `MidiInputListener` and the Tuner's
    /// `TuningUpdateSignaller` (all wired in `new`).
    continuum_protocol: Arc<dyn IContinuumProtocol>,
    /// Weak self-reference used to hand an `Arc<Mutex<Presenter>>` to the OSC layer and the
    /// protocol listener as their callback target. Weak, not Arc, to avoid a reference cycle. Set
    /// by `init`.
    presenter_weak: Weak<Mutex<Presenter>>,
}

impl Presenter {
    /// `timeout_millis` is the number of milliseconds to wait for a tuning update confirmation.
    /// It can be much shorter in tests.
    pub fn new(callbacks: Arc<dyn IUiMethods>, timeout_millis: u16) -> Self {
        // The output MIDI connection is shared between the MidiManager (which connects and
        // disconnects it) and the MidiSender (which writes to it). Create it here and inject it
        // into both, replacing the former OUTPUT_CONNECTION global.
        let output: SharedOutput = Arc::new(Mutex::new(None));
        // The ContinuumProtocol is created here and injected three ways: into the MidiManager (as
        // its raw MidiInputListener), into the Tuner (as its TuningUpdateSignaller), and kept on the
        // Presenter (as its IContinuumProtocol). One shared Arc keeps the Tuner's tuning_status
        // write visible to the protocol's confirmation logic.
        let continuum_protocol = Arc::new(ContinuumProtocol::new());
        let midi_sender: SharedMidiSender =
            Arc::new(Mutex::new(Box::new(MidiSender::new(output.clone())) as Box<dyn IMidiSender>));
        let tuner: SharedTuner = Arc::new(Tuner::new());
        tuner.set_midi_sender(midi_sender.clone());
        tuner.set_tuning_signaller(continuum_protocol.clone());
        let presentation = Presentation::new(callbacks);
        let tuning_update_watchdog =
            TuningUpdateWatchdog::new(presentation.clone(), timeout_millis);
        Self {
            presentation,
            tuning_update_watchdog,
            is_preset_reselect: AtomicBool::new(false),
            osc: Box::new(Osc::new()),
            settings: Box::new(Settings::new()),
            tuner,
            midi_manager: Arc::new(Mutex::new(Box::new(MidiManager::new(
                output, continuum_protocol.clone())) as Box<dyn IMidiManager + Send>)),
            midi_sender,
            continuum_protocol,
            presenter_weak: Weak::new(),
        }
    }

    pub fn init(&mut self, self_arc: &SharedPresenter) {
        trace!("init");
        // Record a weak self-reference so the MIDI/OSC layers can be given an
        // `Arc<Mutex<Presenter>>` callback target without a global singleton.
        self.presenter_weak = Arc::downgrade(self_arc);
        // Register this Presenter as the protocol's semantic listener (Weak, to avoid a cycle).
        let listener: Arc<dyn ContinuumProtocolListener> = self_arc.clone();
        self.continuum_protocol.set_listener(Arc::downgrade(&listener));
        let main_window_x: i32;
        let main_window_y: i32;
        let osc_listening_port: u16;
        let pitch_table: u8;
        let input_device_name: String;
        let output_device_name: String;
        let override_rounding_initial: bool;
        let override_rounding_rate: bool;
        let rounding_rate: u8;
        trace!("init: Reading settings");
        match self.settings.read_from_file() {
            Ok(_) => {
                main_window_x = self.settings.main_window_x();
                main_window_y = self.settings.main_window_y();
                input_device_name = self.settings.midi_input_device().to_string();
                output_device_name = self.settings.midi_output_device().to_string();
                osc_listening_port = {
                    if Osc::listening_ports().contains(&self.settings.osc_listening_port()) {
                        self.settings.osc_listening_port()
                    } else {
                        Osc::default_listening_port()
                    }
                };
                pitch_table = {
                    if Tuner::pitch_tables().contains(&self.settings.pitch_table()) {
                        self.settings.pitch_table()
                    } else {
                        Tuner::default_pitch_table()
                    }
                };
                override_rounding_initial = self.settings.override_rounding_initial();
                override_rounding_rate = self.settings.override_rounding_rate();
                rounding_rate = self.settings.rounding_rate();
            }
            Err(err) => {
                self.presentation.show_error(&err.to_string());
                return;
            }
        }
        trace!("init: Getting midi");
        self.presentation.set_main_window_position(main_window_x, main_window_y);
        let mut midi = self.midi_manager.lock().unwrap();
        midi.init(&input_device_name, &output_device_name);
        drop(midi); // Release MIDI lock before calling device_names which needs to acquire it
        let input_strategy = InputStrategy::new();
        let output_strategy = OutputStrategy::new();
        trace!("init: Getting input device names");
        let input_device_names = self.device_names(&input_strategy);
        trace!("init: Got {} input device names", input_device_names.len());
        trace!("init: About to call callbacks.set_devices_model");
        self.presentation.set_devices_model(&input_device_names, &input_strategy);
        trace!("init: Called callbacks.set_devices_model");
        trace!("init: Setting output devices model");
        self.presentation.set_devices_model(&self.device_names(&output_strategy), &output_strategy);
        trace!("init: Connecting initial MIDI devices");
        self.connect_initial_device(&output_strategy);
        // Don't start listening to MIDI until we are able to send MIDI.
        if self.midi_manager.lock().unwrap().is_output_device_connected() {
            trace!("init: Connecting input device");
            self.connect_initial_device(&input_strategy);
        }
        self.osc.set_listening_port(osc_listening_port);
        self.presentation.set_selected_osc_listening_port_index(Osc::listening_port_index() as i32);
        trace!("init: Configuring tuner");
        self.tuner.init(pitch_table);
        self.presentation.set_selected_pitch_table_index(self.tuner.pitch_table_index() as i32);
        self.tuner.set_override_rounding_initial(override_rounding_initial);
        self.tuner.set_override_rounding_rate(override_rounding_rate);
        self.tuner.set_rounding_rate(rounding_rate);
        self.presentation.set_override_rounding_initial(override_rounding_initial);
        self.presentation.set_override_rounding_rate(override_rounding_rate);
        self.presentation.set_rounding_rate(rounding_rate);
        if self.midi_manager.lock().unwrap().are_devices_connected() {
            trace!("init: Starting instrument connection monitor");
            self.start_instrument_connection_monitor();
        }
        trace!("init: Done");
    }

    fn start_instrument_connection_monitor(&mut self) {
        trace!("start_instrument_connection_monitor");
        self.presentation.checking_instrument_connection();
        self.midi_manager.lock().unwrap().start_instrument_connection_monitor();
        trace!("start_instrument_connection_monitor: Instrument connection monitor started");
    }

    pub fn close(&mut self, main_window_x: i32, main_window_y: i32) -> Result<(), Box<dyn Error>> {
        self.midi_manager.lock().unwrap_or_else(|e| e.into_inner()).close();
        self.osc.stop();
        self.settings.set_main_window_x(main_window_x);
        self.settings.set_main_window_y(main_window_y);
        if let Err(err) = self.settings.write_to_file() {
            self.presentation.show_error(&err.to_string());
            return Err(err)
        };
        Ok(())
    }

    fn connect_initial_device(&mut self, device_strategy: &dyn DeviceStrategy) {
        trace!("connect_initial_port: {:?}", device_strategy.device_type());
        let shared_midi = self.midi_manager.clone();
        let maybe_index = {
            let midi = shared_midi.lock().unwrap();
            midi.io(device_strategy).device().as_ref()
                .map(|device| device.index())
        };
        if let Some(index) = maybe_index {
            trace!("connect_initial_port: Setting selected port index to {}", index);
            self.presentation.set_selected_device_index(index, device_strategy);
            self.connect_selected_device(&shared_midi, device_strategy);
        } else {
            self.presentation.no_device_connected(device_strategy);
            self.presentation.show_warning(device_strategy.msg_connect());
            self.presentation.focus_device(device_strategy);
        }
    }

    pub fn connect_device(&mut self, device_strategy: &dyn DeviceStrategy) {
        trace!("connect_device");
        let shared_midi = self.midi_manager.clone();
        let device_strategy = device_strategy.clone_box();
        trace!("connect_device: Stopping OSC and instrument connection monitor");
        self.stop_osc_and_instrument_connection_monitor();
        trace!("connect_device: Showing PitchGrid disconnected");
        self.presentation.disconnected_from_pitchgrid();
        trace!("connect_device: Connecting selected port");
        self.connect_selected_device(&shared_midi, &*device_strategy);
        trace!("connect_device: Getting port");
        let device_name_opt: Option<String> = shared_midi.lock().unwrap()
            .io(&*device_strategy)
            .device()
            .map(|p| p.name().to_string());
        trace!("connect_device: Got port");
        if let Some(device_name) = device_name_opt {
            self.presentation.show_info(device_strategy.msg_connected(&device_name));
            if self.midi_manager.lock().unwrap().are_devices_connected() {
                trace!("connect_device: Starting instrument connection monitor");
                self.start_instrument_connection_monitor();
            } else {
                let other_device_strategy =
                    device_strategy.other_device_strategy();
                self.presentation.show_warning(other_device_strategy.msg_connect());
            }
        }
        trace!("connect_device: Done");
    }

    fn connect_selected_device(&mut self, shared_midi: &SharedMidiManager,
                               device_strategy: &dyn DeviceStrategy) {
        trace!("connect_selected_device: {:?}", device_strategy.device_type());
        let index = self.presentation.get_selected_device_index(device_strategy);
        // No selection (e.g. an empty device list) leaves the combobox at -1, which `UiMethods`
        // converts to `usize::MAX`. That is handled silently by the `device_names().get(index)`
        // guard below: clicking Connect with nothing to connect to simply does nothing.
        trace!("connect_selected_device: Selected port index = {}", index);
        let ui_action: Result<String, String> = {
            trace!("connect_selected_device: Getting midi.");
            let mut midi = shared_midi.lock().unwrap();
            trace!("connect_selected_device: Got midi.");
            let Some(name) = midi.io(device_strategy).device_names().get(index).cloned()
            else {
                return;
            };
            match midi.connect_device(index, device_strategy) {
                Ok(()) => Ok(name),
                Err(err) => Err(err.to_string()),
            }
        };
        match ui_action {
            Ok(name) => {
                self.presentation.connected_device(&name, device_strategy);
                // Persist the connected device so it is reselected on next startup. Saved only
                // here, where a real device has just connected (never DEVICE_NONE), so the player
                // can close the application and reopen it later — when a temporarily unavailable
                // device is available again (for example a USB-MIDI cable that was unplugged) —
                // and still have the same device automatically selected and connected on startup.
                device_strategy.set_device_setting(&mut *self.settings, &name);
            }
            Err(message) => {
                self.presentation.no_device_connected(device_strategy);
                self.presentation.show_error(&message);
            }
        }
    }

    fn device_names(&self, device_strategy: &dyn DeviceStrategy) -> Vec<String> {
        self.midi_manager.lock().unwrap().io(device_strategy).device_names()
    }

    pub fn refresh_devices(&mut self, device_strategy: &dyn DeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        self.stop_osc_and_instrument_connection_monitor();
        let device_name = device_strategy.device_setting(&*self.settings).to_string();
        self.midi_manager.lock().unwrap().refresh_devices(&device_name, &*device_strategy);
        self.presentation.disconnected_from_pitchgrid();
        self.presentation.set_devices_model(&self.device_names(&*device_strategy), &*device_strategy);
        self.presentation.no_device_connected(&*device_strategy);
        self.presentation.show_warning(device_strategy.msg_refreshed_reconnect());
    }

    /// Replaces the default Osc instance for testing.
    pub fn set_osc(&mut self, osc: Box<dyn IOsc>) { self.osc = osc; }

    /// Replaces the default Settings instance for testing.
    pub fn set_settings(&mut self, settings: Box<dyn ISettings>) { self.settings = settings; }

    /// Replaces the default Tuner instance for testing.
    pub fn set_tuner(&mut self, tuner: SharedTuner) {
        self.tuner = tuner;
    }

    /// Replaces the default MidiManager instance for testing.
    pub fn set_midi_manager(&mut self, midi: Box<dyn IMidiManager + Send>) {
        self.midi_manager = Arc::new(Mutex::new(midi));
    }

    /// Replaces the default ContinuumProtocol instance for testing.
    pub fn set_continuum_protocol(&mut self, protocol: Arc<dyn IContinuumProtocol>) {
        self.continuum_protocol = protocol;
    }

    /// Returns an `Arc` to this Presenter for use as a MIDI/OSC callback target.
    /// Relies on `init` having recorded the weak self-reference.
    fn clone_presenter(&self) -> SharedPresenter {
        self.presenter_weak.upgrade()
            .expect("presenter_weak not set; init() must run before clone_presenter()")
    }

    /// Sets the root frequency override and sends it to the instrument,
    /// if the instrument and PitchGrid are both connected.
    /// We probably don't need a setting for this.
    /// The player should have to choose an override, if required, on startup.
    pub fn set_root_freq_override(&mut self, index: usize) {
        let send_tuning = self.midi_manager.lock().unwrap().is_connected_and_receiving()
            && self.continuum_protocol.has_downloaded_init_data()
            && self.osc.is_pitchgrid_connected();
        if send_tuning {
            self.presentation.updating_root_freq_override();
        }
        self.tuner.set_root_freq_override_note_no(index, send_tuning);
    }

    pub fn set_override_rounding_initial(&mut self, value: bool) {
        self.tuner.set_override_rounding_initial(value);
        self.settings.set_override_rounding_initial(value);
    }

    pub fn set_override_rounding_rate(&mut self, value: bool) {
        self.tuner.set_override_rounding_rate(value);
        self.settings.set_override_rounding_rate(value);
    }

    pub fn set_rounding_rate(&mut self, rate: u8) {
        self.tuner.set_rounding_rate(rate);
        self.settings.set_rounding_rate(rate);
    }

    pub fn set_osc_listening_port(&mut self, index: usize) {
        let osc_listening_port = Osc::listening_ports()[index];
        self.osc.set_listening_port(osc_listening_port);
        self.settings.set_osc_listening_port(osc_listening_port);
    }

    pub fn set_pitch_table(&mut self, index: usize) {
        let pitch_table = Tuner::pitch_tables()[index];
        self.tuner.set_pitch_table(pitch_table);
        self.settings.set_pitch_table(pitch_table);
    }

    fn on_data_download_completed(&mut self) {
        debug!("on_data_download_completed");
        if self.midi_manager.lock().unwrap().is_connected_and_receiving()
                && !self.osc.is_running() {
            debug!("on_data_download_completed: Starting OSC");
            self.start_osc();
            self.presentation.opening_pitchgrid_connection();
        }
    }

    fn on_data_download_started(&mut self) {
        trace!("on_data_download_started");
        self.presentation.awaiting_data_download();
    }

    fn on_devices_connected_changed(&mut self) {
        trace!("on_devices_connected_changed");
        if self.midi_manager.lock().unwrap().are_devices_connected() {
            return;
        }
        // At least one MIDI port is not connected. Stop OSC if running.
        trace!("on_devices_connected_changed: Ports are not connected.");
        if self.osc.is_running() {
            trace!("on_devices_connected_changed: Stopping OSC");
            self.stop_osc_and_show_pitchgrid_status();
            self.presentation.instrument_disconnected();
        }
        self.presentation.pitchgrid_connection_closed();
    }

    fn on_new_preset_selected(&self) {
        debug!("on_new_preset_selected");
        // Nothing to resend until a tuning has been generated and sent at least once;
        // in that case `send_current_preset_update` is a no-op.
        if !self.tuner.has_data() {
            return;
        }
        // No progress message is shown here, deliberately. Unlike a fresh tuning
        // (on_tuning_received), this resend uses send_tuning_update(false): it does NOT upload the
        // 128-key table, so it completes almost instantly. A progress message would be overwritten
        // by the confirmation before the UI could paint it, so it would never be seen. Instead we
        // flag the resend so that its confirmation (on_tuning_updated) shows PRESET_TUNING_LOADED
        // rather than the generic INSTRUMENT_TUNING_UPDATED.
        self.is_preset_reselect.store(true, Ordering::Relaxed);
        self.tuner.send_current_preset_update();
        debug!("on_new_preset_selected: Updated");
    }

    /// Started receiving data from the instrument.
    fn on_receiving_data_started(&mut self) {
        trace!("on_receiving_data_started");
        // The input device is connected, as we are receiving data from the instrument.
        // But the output device might not be, in which case we can't send data to the instrument
        // and should not overwrite the "Connect MIDI output device" warning message that should
        // already be displayed.
        if self.midi_manager.lock().unwrap().are_devices_connected() {
            trace!("on_receiving_data_started: Waiting for data download to complete.");
            self.presentation.waiting_for_data_download();
        }
    }

    /// Stopped receiving data from the instrument.
    fn on_receiving_data_stopped(&mut self) {
        trace!("on_receiving_data_stopped");
        if self.osc.is_running() {
            debug!("on_receiving_data_stopped: Stopping OSC");
            self.stop_osc_and_show_pitchgrid_status();
        }
        if self.midi_manager.lock().unwrap().are_devices_connected() {
            trace!("on_receiving_data_stopped: Showing instrument not connected warning");
            self.presentation.instrument_not_connected();
        }
    }

    fn on_tuning_updated(&mut self) {
        debug!("on_tuning_updated");
        // Stop waiting for tuning update to be confirmed.
        self.tuning_update_watchdog.cancel();
        self.tuner.on_tuning_updated();
        // If there's no tuning data, the displayed tuning data will be blanked.
        self.presentation.show_tuning(self.tuner.formatted_tuning_params(), self.tuner.is_root_freq_overridden());
        if !self.tuner.has_data() {
            // Could be tuning updated when an instrument preset is loaded
            // while PitchGrid is not connected.
            return;
        }
        // A resend triggered by a new preset selection gets its own confirmation; a fresh tuning
        // gets the generic one. swap() reads and clears the flag in one step.
        if self.is_preset_reselect.swap(false, Ordering::Relaxed) {
            self.presentation.preset_tuning_loaded();
        } else {
            self.presentation.instrument_tuning_updated();
        }
    }

    fn on_updating_tuning(&mut self) {
        debug!("on_updating_tuning");
        self.tuning_update_watchdog.start();
    }

    fn start_osc(&mut self) {
        let callbacks = self.clone_presenter();
        self.osc.start(callbacks);
    }

    fn stop_osc_and_instrument_connection_monitor(&mut self) {
        trace!("stop_osc_and_instrument_connection_monitor");
        self.midi_manager.lock().unwrap().stop_instrument_connection_monitor();
        self.stop_osc_and_show_pitchgrid_status();
        trace!("stop_osc_and_instrument_connection_monitor: Done");
    }

    /// Stops PitchGrid OSC, blanks the displayed tuning and shows a PitchGrid status message.
    /// When the application is reconnected to the instrument, OSC will be restarted, which will
    /// force PitchGrid to send the latest tuning.
    fn stop_osc_and_show_pitchgrid_status(&self) {
        debug!("stop_osc_and_show_pitchgrid_status");
        self.osc.stop();
        self.remove_data();
        if !self.midi_manager.lock().unwrap().are_devices_connected() {
            self.presentation.cannot_update_tuning_connect();
        } else if !self.midi_manager.lock().unwrap().is_receiving_data() {
            self.presentation.cannot_update_tuning_lost();
        }
    }

    /// Blanks the displayed tuning and removes tuning data from the tuner.
    fn remove_data(&self) {
        debug!("remove_data");
        self.tuner.remove_data();
        // As we've removed tuning data, the displayed tuning data will be blanked.
        self.presentation.show_tuning(self.tuner.formatted_tuning_params(), self.tuner.is_root_freq_overridden());
    }
}

impl ContinuumProtocolListener for Mutex<Presenter> {
    fn on_download_completed(&self) {
        self.lock().unwrap().on_data_download_completed();
    }

    fn on_download_started(&self) {
        self.lock().unwrap().on_data_download_started();
    }

    fn on_new_preset_selected(&self) {
        self.lock().unwrap().on_new_preset_selected();
    }

    fn on_devices_connected_changed(&self) {
        self.lock().unwrap().on_devices_connected_changed();
    }

    fn on_receiving_data_started(&self) {
        self.lock().unwrap().on_receiving_data_started();
    }

    fn on_receiving_data_stopped(&self) {
        self.lock().unwrap().on_receiving_data_stopped();
    }

    fn on_tuning_updated(&self) {
        self.lock().unwrap().on_tuning_updated();
    }

    fn on_updating_tuning(&self) {
        self.lock().unwrap().on_updating_tuning();
    }
}

impl OscCallbacks for Mutex<Presenter> {
    fn on_pitchgrid_connected_changed(&self) {
        trace!("OscCallbacks for Mutex<Presenter>.on_pitchgrid_connected_changed");
        let presenter = self.lock().unwrap();
        presenter.on_pitchgrid_connected_changed();
    }

    fn on_tuning_received(&self, tuning_params: TuningParams) {
        trace!("OscCallbacks for Mutex<Presenter>.on_tuning_received");
        let presenter = self.lock().unwrap();
        presenter.on_tuning_received(tuning_params);
    }
}

impl OscCallbacks for Presenter {
    fn on_pitchgrid_connected_changed(&self) {
        trace!("on_pitchgrid_connected_changed");
        if self.osc.is_pitchgrid_connected() {
            debug!("on_pitchgrid_connected_changed: Showing tuning");
            self.presentation.show_tuning(self.tuner.formatted_tuning_params(), self.tuner.is_root_freq_overridden());
            trace!("on_pitchgrid_connected_changed: Showing PitchGrid is connected");
            self.presentation.pitchgrid_connected();
            trace!("on_pitchgrid_connected_changed: PitchGrid and instrument are connected");
            self.presentation.pitchgrid_and_instrument_connected();
        } else {
            debug!("on_pitchgrid_connected_changed: PitchGrid is not connected");
            self.remove_data();
            self.presentation.pitchgrid_not_connected();
            self.presentation.awaiting_pitchgrid_connection();
        }
    }

    fn on_tuning_received(&self, tuning_params: TuningParams) {
        trace!("on_tuning_received");
        // A fresh tuning is not a preset reselect. Clear any flag left set by a preset reselect
        // whose confirmation never arrived, so this tuning's confirmation isn't mislabelled.
        self.is_preset_reselect.store(false, Ordering::Relaxed);
        trace!("on_tuning_received: {tuning_params:?}");
        if self.midi_manager.lock().unwrap().is_connected_and_receiving() {
            debug!("on_tuning_received: Showing Updating instrument tuning");
            self.presentation.updating_instrument_tuning();
            debug!("on_tuning_received: Updating instrument tuning");
            self.tuner.on_tuning_received(tuning_params);
        } else {
            self.stop_osc_and_show_pitchgrid_status();
        }
    }
}

type SharedPresenter = Arc<Mutex<Presenter>>;
