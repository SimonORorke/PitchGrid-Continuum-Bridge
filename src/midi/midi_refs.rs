use midir::MidiOutputConnection;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use crate::i_midi::MidiCallbacks;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum DownloadStatus {
    NotChecked,
    Waiting,
    BeginUserNames,
    EndUserNames,
    BeginSysNames,
    EndSysNames,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TuningStatus {
    None,
    Tuning,
    // RequestedPresetUpdate,
}

static CALLBACKS: Mutex<Option<Arc<dyn MidiCallbacks>>> = Mutex::new(None);
static DOWNLOAD_STATUS: Mutex<DownloadStatus> = Mutex::new(DownloadStatus::NotChecked);
static DOWNLOAD_WAIT_START_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_MESSAGE_RECEIVED_TIME: Mutex<Option<Instant>> = Mutex::new(None);
static OUTPUT_CONNECTION: Mutex<Option<MidiOutputConnection>> = Mutex::new(None);
static TUNING_STATUS: Mutex<TuningStatus> = Mutex::new(TuningStatus::None);

pub(super) fn set_callbacks(callbacks: Arc<dyn MidiCallbacks>) {
    *CALLBACKS.lock().unwrap() = Some(callbacks);
}

pub(super) fn callbacks() -> Option<Arc<dyn MidiCallbacks>> {
    CALLBACKS.lock().unwrap().clone()
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

pub(super) fn output_connection() -> &'static Mutex<Option<MidiOutputConnection>> {
    &OUTPUT_CONNECTION
}

pub(super) fn tuning_status() -> &'static Mutex<TuningStatus> { &TUNING_STATUS }
