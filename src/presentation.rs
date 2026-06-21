use std::sync::Arc;
use log::{debug, trace};
use crate::global::MessageType;
use crate::device_strategy::DeviceStrategy;
use crate::i_ui_methods::IUiMethods;
use crate::tuning_params::FormattedTuningParams;

/// The view-facing facade the `Presenter` speaks through.
///
/// It owns the injected `IUiMethods` view seam and every user-facing message string (the `const`s
/// below), exposing intention-named methods (e.g. `disconnected_from_pitchgrid`) so that each
/// string and its severity live in one place rather than being chosen at the call site. It decides
/// *nothing* about when to show a message — that is the Presenter's job; this type only formats and
/// forwards to the view.
///
/// It is a concrete type, not an `I`-prefixed service interface: tests mock the underlying
/// `IUiMethods`, so the facade needs no interface of its own (see `ITuner`'s doc comment for the
/// two-tier trait-naming rule). `Clone` is cheap — it wraps a single `Arc` — so the
/// `TuningUpdateWatchdog`'s background thread can own a copy.
#[derive(Clone)]
pub(crate) struct Presentation {
    ui_methods: Arc<dyn IUiMethods>,
}

impl Presentation {
    pub(crate) fn new(ui_methods: Arc<dyn IUiMethods>) -> Self {
        Self { ui_methods }
    }

    // --- Structural view setup (thin passthroughs to the view) ---

    pub(crate) fn set_main_window_position(&self, x: i32, y: i32) {
        self.ui_methods.set_main_window_position(x, y);
    }

    pub(crate) fn set_devices_model(&self, device_names: &[String],
                                    device_strategy: &dyn DeviceStrategy) {
        self.ui_methods.set_devices_model(device_names, device_strategy);
    }

    pub(crate) fn get_selected_device_index(&self, device_strategy: &dyn DeviceStrategy) -> usize {
        self.ui_methods.get_selected_device_index(device_strategy)
    }

    pub(crate) fn set_selected_device_index(&self, index: usize,
                                            device_strategy: &dyn DeviceStrategy) {
        self.ui_methods.set_selected_device_index(index, device_strategy);
    }

    pub(crate) fn focus_device(&self, device_strategy: &dyn DeviceStrategy) {
        self.ui_methods.focus_device(device_strategy);
    }

    pub(crate) fn set_selected_osc_listening_port_index(&self, index: i32) {
        self.ui_methods.set_selected_osc_listening_port_index(index);
    }

    pub(crate) fn set_selected_pitch_table_index(&self, index: i32) {
        self.ui_methods.set_selected_pitch_table_index(index);
    }

    pub(crate) fn set_override_rounding_initial(&self, value: bool) {
        self.ui_methods.set_override_rounding_initial(value);
    }

    pub(crate) fn set_override_rounding_rate(&self, value: bool) {
        self.ui_methods.set_override_rounding_rate(value);
    }

    pub(crate) fn set_rounding_rate(&self, rate: u8) {
        self.ui_methods.set_rounding_rate(rate);
    }

    // --- Tuning table display ---

    pub(crate) fn show_tuning(&self, tuning: FormattedTuningParams, is_root_freq_overridden: bool) {
        self.ui_methods.show_tuning(tuning, is_root_freq_overridden);
    }

    // --- Connected-device name (Info for a real device, Warning for none) ---

    pub(crate) fn connected_device(&self, device_name: &str,
                                   device_strategy: &dyn DeviceStrategy) {
        self.ui_methods.show_connected_device_name(device_name, MessageType::Info, device_strategy);
    }

    pub(crate) fn no_device_connected(&self, device_strategy: &dyn DeviceStrategy) {
        self.ui_methods.show_connected_device_name(
            DEVICE_NONE, MessageType::Warning, device_strategy);
    }

    // --- Generic messages whose text is dynamic (a Result error or a DeviceStrategy string) ---

    pub(crate) fn show_error(&self, message: &str) {
        self.ui_methods.show_message(message, MessageType::Error);
    }

    pub(crate) fn show_info(&self, message: &str) {
        self.ui_methods.show_message(message, MessageType::Info);
    }

    pub(crate) fn show_warning(&self, message: &str) {
        trace!("show_warning: {message}");
        self.ui_methods.show_message(message, MessageType::Warning);
    }

    // --- Intention-named status messages (string + severity owned here) ---

    pub(crate) fn checking_instrument_connection(&self) {
        self.show_info(CHECKING_INSTRUMENT_CONNECTION);
    }

    pub(crate) fn awaiting_data_download(&self) {
        self.show_info(AWAITING_DATA_DOWNLOAD_COMPLETION);
    }

    pub(crate) fn waiting_for_data_download(&self) {
        self.show_info(WAITING_FOR_DATA_DOWNLOAD);
    }

    pub(crate) fn opening_pitchgrid_connection(&self) {
        self.show_info(OPENING_PITCHGRID_CONNECTION);
    }

    pub(crate) fn pitchgrid_and_instrument_connected(&self) {
        self.show_info(PITCHGRID_AND_INSTRUMENT_CONNECTED);
    }

    pub(crate) fn instrument_disconnected(&self) {
        self.show_warning(INSTRUMENT_DISCONNECTED);
    }

    pub(crate) fn instrument_not_connected(&self) {
        self.show_warning(INSTRUMENT_NOT_CONNECTED);
    }

    pub(crate) fn awaiting_pitchgrid_connection(&self) {
        self.show_warning(AWAITING_PITCHGRID_CONNECTION);
    }

    pub(crate) fn updating_root_freq_override(&self) {
        self.show_pitchgrid_status(UPDATING_ROOT_FREQ_OVERRIDE, MessageType::Info);
    }

    pub(crate) fn updating_instrument_tuning(&self) {
        self.show_pitchgrid_status(UPDATING_INSTRUMENT_TUNING, MessageType::Info);
    }

    pub(crate) fn preset_tuning_loaded(&self) {
        debug!("Showing {PRESET_TUNING_LOADED}");
        self.show_pitchgrid_status(PRESET_TUNING_LOADED, MessageType::Info);
    }

    pub(crate) fn instrument_tuning_updated(&self) {
        debug!("Showing {INSTRUMENT_TUNING_UPDATED}");
        self.show_pitchgrid_status(INSTRUMENT_TUNING_UPDATED, MessageType::Info);
    }

    pub(crate) fn pitchgrid_connected(&self) {
        trace!("pitchgrid_connected: Showing PitchGrid OSC is connected");
        self.show_pitchgrid_status(PITCHGRID_OSC_CONNECTED, MessageType::Info);
    }

    pub(crate) fn disconnected_from_pitchgrid(&self) {
        self.show_pitchgrid_status(DISCONNECTED_FROM_PITCHGRID, MessageType::Warning);
    }

    pub(crate) fn pitchgrid_connection_closed(&self) {
        self.show_pitchgrid_status(PITCHGRID_CONNECTION_CLOSED, MessageType::Warning);
    }

    pub(crate) fn pitchgrid_not_connected(&self) {
        self.show_pitchgrid_status(PITCHGRID_NOT_CONNECTED, MessageType::Error);
    }

    pub(crate) fn cannot_update_tuning_connect(&self) {
        self.show_pitchgrid_status(CANNOT_UPDATE_TUNING_CONNECT, MessageType::Error);
    }

    pub(crate) fn cannot_update_tuning_lost(&self) {
        self.show_pitchgrid_status(CANNOT_UPDATE_TUNING_LOST, MessageType::Error);
    }

    pub(crate) fn tuning_update_not_confirmed(&self) {
        self.ui_methods.show_message(INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED, MessageType::Error);
    }

    // --- private ---

    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType) {
        self.ui_methods.show_pitchgrid_status(status, msg_type);
    }
}

// User-facing message strings. Owned here so every string the player sees lives in one module.
// `DEVICE_NONE` is the sentinel shown (and recorded by tests) when no MIDI device is connected.
pub const AWAITING_DATA_DOWNLOAD_COMPLETION: &str = "Awaiting completion of data download from instrument...";
pub const AWAITING_PITCHGRID_CONNECTION: &str = "Awaiting PitchGrid connection...";
pub const CANNOT_UPDATE_TUNING_CONNECT: &str = "Cannot update tuning. Connect instrument input/output.";
pub const CANNOT_UPDATE_TUNING_LOST: &str = "Cannot update tuning. Instrument connection lost.";
pub const CHECKING_INSTRUMENT_CONNECTION: &str = "Checking instrument connection...";
pub const DEVICE_NONE: &str = "[None]";
pub const DISCONNECTED_FROM_PITCHGRID: &str = "Disconnected from PitchGrid because MIDI is not connected";
pub const INSTRUMENT_DISCONNECTED: &str = "Instrument is disconnected; closed PitchGrid connection.";
pub const INSTRUMENT_NOT_CONNECTED: &str = "The instrument is not connected. Waiting for the editor to be \
        opened with this application and the instrument connected to it...";
pub const INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED: &str = "Instrument tuning update has not been \
    confirmed. Ensure that MIDI output is connected to the editor.";
pub const INSTRUMENT_TUNING_UPDATED: &str = "Instrument tuning updated";
pub const OPENING_PITCHGRID_CONNECTION: &str = "Opening PitchGrid connection...";
pub const PITCHGRID_AND_INSTRUMENT_CONNECTED: &str = "PitchGrid and instrument are connected";
pub const PITCHGRID_CONNECTION_CLOSED: &str = "PitchGrid connection closed while instrument disconnected";
pub const PITCHGRID_NOT_CONNECTED: &str = "PitchGrid is not connected. OSC must be enabled in PitchGrid.";
pub const PITCHGRID_OSC_CONNECTED: &str = "PitchGrid OSC is connected";
pub const PRESET_TUNING_LOADED: &str = "Tuning loaded to new instrument preset";
pub const UPDATING_INSTRUMENT_TUNING: &str = "Updating instrument tuning...";
pub const UPDATING_ROOT_FREQ_OVERRIDE: &str = "Updating root frequency override...";
pub const WAITING_FOR_DATA_DOWNLOAD: &str =
    "Waiting (maximum 6 seconds) for possible initial data download from instrument...";
