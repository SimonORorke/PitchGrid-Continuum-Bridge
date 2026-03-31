use midir::MidiOutputConnection;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::mpsc;
use std::time::Instant;

pub(super) type Callbacks = Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum DownloadStatus {
    None,
    Checking,
    BeginUserNames,
    EndUserNames,
    BeginSysNames,
    EndSysNames,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PresetSelectStatus {
    None,
    BankH,
    // Program,
}

static DOWNLOAD_COMPLETED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();
static DOWNLOAD_MONITOR_STOPPER_SENDER: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static DOWNLOAD_STARTED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();
static DOWNLOAD_STATUS: Mutex<DownloadStatus> = Mutex::new(DownloadStatus::None);
static DOWNLOAD_WAIT_START_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_MESSAGE_RECEIVED_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static NEW_PRESET_SELECTED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();
static OUTPUT_CONNECTION: Mutex<Option<MidiOutputConnection>> = Mutex::new(None);
static PORTS_CONNECTED_CHANGED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();
static PRESET_SELECT_STATUS: Mutex<PresetSelectStatus> = Mutex::new(PresetSelectStatus::None);
static RECEIVING_DATA_STARTED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();
static RECEIVING_DATA_STOPPED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();
static TUNING_UPDATED_CALLBACKS: OnceLock<Callbacks> = OnceLock::new();

pub(super) fn download_completed_callbacks() -> &'static Callbacks {
    DOWNLOAD_COMPLETED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub(super) fn download_started_callbacks() -> &'static Callbacks {
    DOWNLOAD_STARTED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub(super) fn download_monitor_stopper_sender() -> &'static Mutex<Option<mpsc::Sender<()>>> {
    &DOWNLOAD_MONITOR_STOPPER_SENDER
}

pub(super) fn download_status() -> &'static Mutex<DownloadStatus> {
    &DOWNLOAD_STATUS
}

pub(super) fn download_wait_start_time() -> &'static Mutex<Option<Instant>> {
    &DOWNLOAD_WAIT_START_TIME
}

pub(super) fn last_message_received_time() -> &'static Mutex<Option<Instant>> {
    &LAST_MESSAGE_RECEIVED_TIME
}

pub(super) fn new_preset_selected_callbacks() -> &'static Callbacks {
    NEW_PRESET_SELECTED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub(super) fn output_connection() -> &'static Mutex<Option<MidiOutputConnection>> {
    &OUTPUT_CONNECTION
}

pub(super) fn ports_connected_changed_callbacks() -> &'static Callbacks {
    PORTS_CONNECTED_CHANGED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub(super) fn preset_select_status() -> &'static Mutex<PresetSelectStatus> {
    &PRESET_SELECT_STATUS
}

pub(super) fn receiving_data_started_callbacks() -> &'static Callbacks {
    RECEIVING_DATA_STARTED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub(super) fn receiving_data_stopped_callbacks() -> &'static Callbacks {
    RECEIVING_DATA_STOPPED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub(super) fn tuning_updated_callbacks() -> &'static Callbacks {
    TUNING_UPDATED_CALLBACKS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}
