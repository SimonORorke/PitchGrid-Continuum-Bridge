use crate::i_continuum_protocol::{
    ContinuumProtocolListener, IContinuumProtocol, TuningUpdateSignaller};
use crate::i_midi_manager::MidiInputListener;
use crate::tuner::Tuner;
use log::{debug, trace};
use midly::{MidiMessage, live::LiveEvent};
use std::sync::{Arc, Mutex, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
enum DownloadStatus {
    NotChecked,
    Waiting,
    BeginUserNames,
    EndUserNames,
    BeginSysNames,
    EndSysNames,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TuningStatus {
    None,
    Tuning,
    // RequestedPresetUpdate,
}

/// Interprets the Continuum MIDI protocol. It consumes the raw messages and connection-lifecycle
/// events raised by the generic `MidiManager` (it is the manager's `MidiInputListener`) and raises
/// the resulting semantic events to the `Controller` (it holds the `Controller` as a
/// `ContinuumProtocolListener`). It owns the protocol state formerly held in the `midi_refs`
/// statics (and, in 3b, in `MidiState`).
///
/// One instance is shared three ways, all wired in `Controller::new`: the `MidiManager` holds it as
/// its raw listener, the `Tuner` holds it as its `TuningUpdateSignaller`, and the `Controller`
/// holds it as its `IContinuumProtocol`.
pub struct ContinuumProtocol {
    /// The semantic listener (the `Controller`). Weak to avoid a reference cycle; set by
    /// `Controller::init`. Replaces the former `CALLBACKS` global.
    listener: Mutex<Option<Weak<dyn ContinuumProtocolListener>>>,
    download_status: Mutex<DownloadStatus>,
    download_wait_start_time: Mutex<Option<Instant>>,
    is_monitoring_download: AtomicBool,
    tuning_status: Mutex<TuningStatus>,
    /// This layer's own clock for the 200 ms download burst-gap detection, kept separate from the
    /// `MidiManager`'s connection-liveness timestamp (the two were one static before the split).
    last_message_time: Mutex<Option<Instant>>,
}

impl ContinuumProtocol {
    pub fn new() -> Self {
        Self {
            listener: Mutex::new(None),
            download_status: Mutex::new(DownloadStatus::NotChecked),
            download_wait_start_time: Mutex::new(None),
            is_monitoring_download: AtomicBool::new(false),
            tuning_status: Mutex::new(TuningStatus::None),
            last_message_time: Mutex::new(None),
        }
    }

    /// The semantic listener, if it is still alive. Mirrors the former `callbacks()` accessor.
    fn listener(&self) -> Option<Arc<dyn ContinuumProtocolListener>> {
        self.listener.lock().unwrap().as_ref().and_then(|weak| weak.upgrade())
    }

    fn on_init_data_download_completed(&self) {
        trace!("ContinuumProtocol.on_init_data_download_completed: Stopping download monitor");
        self.is_monitoring_download.store(false, Ordering::Relaxed);
        *self.download_status.lock().unwrap() = DownloadStatus::Complete;
        if let Some(listener) = self.listener() {
            rayon::spawn(move || listener.on_download_completed());
        }
    }
}

impl IContinuumProtocol for ContinuumProtocol {
    fn has_downloaded_init_data(&self) -> bool {
        *self.download_status.lock().unwrap() == DownloadStatus::Complete
    }

    fn set_listener(&self, listener: Weak<dyn ContinuumProtocolListener>) {
        *self.listener.lock().unwrap() = Some(listener);
    }
}

impl TuningUpdateSignaller for ContinuumProtocol {
    fn on_updating_tuning(&self) {
        debug!("ContinuumProtocol.on_updating_tuning");
        *self.tuning_status.lock().unwrap() = TuningStatus::Tuning;
        if let Some(listener) = self.listener() {
            rayon::spawn(move || listener.on_updating_tuning());
        }
    }
}

impl MidiInputListener for ContinuumProtocol {
    fn on_message(&self, message: &[u8]) {
        // Download-monitor timing (formerly the tail of `MidiState::log_message_received_time`),
        // keyed off this layer's own message clock. The first-message setup that used to live here
        // now happens in `on_receiving_data_started`.
        let now = Instant::now();
        let prev_message_time = {
            let mut last = self.last_message_time.lock().unwrap();
            let prev = *last;
            *last = Some(now);
            prev
        };
        if self.is_monitoring_download.load(Ordering::Relaxed) {
            if let Some(prev) = prev_message_time
                && now.duration_since(prev).as_millis() >= 200
            {
                // The initial data download consists of many messages in quick succession.
                // Or this could be some other burst of messages, such as the heartbeat cluster.
                // Either way, as we have not received any more messages for 200 ms,
                // the burst of messages must have stopped.
                self.on_init_data_download_completed();
            }
        } else if *self.download_status.lock().unwrap() != DownloadStatus::Complete {
            // Check whether it is time to start monitoring the initial data download.
            // We waited 6 seconds (from the first message) to give the download a chance to start.
            if let Some(start) = *self.download_wait_start_time.lock().unwrap()
                && now.duration_since(start).as_secs() >= 6
            {
                trace!("ContinuumProtocol.on_message: Starting download monitor");
                self.is_monitoring_download.store(true, Ordering::Relaxed);
            }
        }
        // Parse + interpret (formerly the body of `MidiState::on_message_received`).
        let event = LiveEvent::parse(message).unwrap();
        if let LiveEvent::Midi { channel, message } = event {
            match message {
                MidiMessage::Controller { controller, value } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 != 16 {
                        return;
                    }
                    // Channel 16: the instrument's control channel for most parameters.
                    if controller != 82 && controller != 111 && controller != 114
                        && controller != 118 {  // Heartbeats ignored
                        trace!("Midi.on_message_received: ch{} cc{} value {}",
                               channel1, u8::from(controller), u8::from(value));
                    }
                    if controller == 51 { // Grid
                        let pitch_table = u8::from(value);
                        trace!("midi.on_message_received: Pitch table {}", pitch_table);
                        // A pitch table has been loaded to the instrument's current preset.
                        // This message is received as part of instrument config,
                        // and when a pitch table update sent to the instrument has been
                        // completed and loaded.
                        let status = *self.tuning_status.lock().unwrap();
                        // Workaround for firmware 10.73 Beta not sending update confirmation
                        // for some presets.
                        if status == TuningStatus::Tuning {
                            // Check that the value is the correct pitch table index
                            // for the tuning this application sent to the instrument:
                            // when a preset is loaded, there will be a Grid message
                            // for the preset's initial tuning table, which will be zero.
                            if pitch_table == Tuner::pitch_table() {
                                debug!("ContinuumProtocol.on_message: Preset's pitch table \
                                            update requested, pitch table no: {}", pitch_table);
                                *self.tuning_status.lock().unwrap() = TuningStatus::None;
                                if let Some(listener) = self.listener() {
                                    rayon::spawn(move || listener.on_tuning_updated());
                                }
                            }
                        }
                        // When the firmware bug is fixed, remove the above workaround
                        // and restore the tuning update confirmation check below.
                        // This will fix the problem described in a comment in
                        // Controller.await_tuning_updated.
                        // match status {
                        //     TuningStatus::None => {}
                        //     TuningStatus::Tuning => {
                        //         // Check that the value is the correct pitch table index
                        //         // for the tuning this application sent to the instrument.
                        //         // When there have been problems at the instrument end,
                        //         // it has sent back a ch16 cc51 message, but with value 0.
                        //         if pitch_table == Tuner::pitch_table() {
                        //             // The editor sends us back what we send to the instrument,
                        //             // as well as what the instrument sends back to us.
                        //             // So we have just requested that the current preset be updated
                        //             // with the new pitch table.
                        //             println!("midi.on_message_received: Preset's pitch table \
                        //                 update requested, pitch table no: {}", pitch_table);
                        //             *tuning_status().lock().unwrap() =
                        //                 TuningStatus::RequestedPresetUpdate;
                        //         }
                        //     }
                        //     TuningStatus::RequestedPresetUpdate => {
                        //         // The instrument has confirmed that the current preset has been
                        //         // updated with the new pitch table.
                        //         // As at firmware 10.73, there is a firmware bug where, for
                        //         // specific presets, the instrument will send back a cc51 message
                        //         // with value 0 instead of the pitch table no we requested.
                        //         // Haken Audio Incident 2335
                        //         // https://github.com/SimonORorke/PitchGrid-Continuum-Bridge/issues/5
                        //         // So we can omit checking the pitch table no here.
                        //         println!("midi.on_message_received: Preset's pitch table \
                        //                 update confirmed, pitch table no: {}", pitch_table);
                        //         *tuning_status().lock().unwrap() = TuningStatus::None;
                        //         Self::call_back(tuning_updated_callbacks().clone());
                        //     }
                        // }
                        return;
                    }
                    if controller == 109 {
                        if value == 40 {
                            trace!("midi.on_message_received: EndSysNames");
                            *self.download_status.lock().unwrap() = DownloadStatus::EndSysNames;
                            return;
                        }
                        if value == 49 {
                            trace!("midi.on_message_received: BeginSysNames");
                            *self.download_status.lock().unwrap() = DownloadStatus::BeginSysNames;
                            if let Some(listener) = self.listener() {
                                rayon::spawn(move || listener.on_download_started());
                            }
                            return;
                        }
                        if value == 54 {
                            trace!("midi.on_message_received: BeginUserNames");
                            *self.download_status.lock().unwrap() = DownloadStatus::BeginUserNames;
                            // If system preset names have been downloaded,
                            // which should only have happened on firmware upgrade,
                            // `on_download_started` will have been called already.
                            // However, doing it again will do no harm,
                            // as it will only result in the same status message being redisplayed.
                            if let Some(listener) = self.listener() {
                                rayon::spawn(move || listener.on_download_started());
                            }
                            return;
                        }
                        if value == 55 {
                            trace!("midi.on_message_received: EndUserNames");
                            *self.download_status.lock().unwrap() = DownloadStatus::EndUserNames;
                        }
                    }
                }
                MidiMessage::ProgramChange { program } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        // When the editor requests a preset load, which can be seen in the
                        // editor's console log but not here, the program number is zero-based.
                        // When the instrument confirms that the preset has been loaded,
                        // which we see here, the program number is one-based.
                        trace!("midi.on_message_received: ProgramChange ch16 program {}", u8::from(program));
                        let download_status = *self.download_status.lock().unwrap();
                        // I don't think this will work if system presets are downloaded.
                        // But it's a rare occurrence; and the user will be able to work around it.
                        if download_status == DownloadStatus::EndUserNames
                            || download_status == DownloadStatus::EndSysNames {
                            trace!("Midi.on_message_received: End of download");
                            self.on_init_data_download_completed();
                            return;
                        }
                        if download_status == DownloadStatus::Complete {
                            // The user is selecting a preset. The editor sends the preset's
                            // zero-based program number after the bank.
                            // For unknown reason, this happens twice when a preset is loaded
                            // from disc.
                            trace!("midi.on_message_received: Program, preset selected");
                            if let Some(listener) = self.listener() {
                                rayon::spawn(move || listener.on_new_preset_selected());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn on_receiving_data_started(&self) {
        // The first message has arrived since monitoring started. We need to wait for the initial
        // data download to the editor to complete, if it did not already happen before we started
        // listening. We judge the download complete either when we receive the last download message
        // or if there's been no data for 0.2 seconds. On the Continuum, the initial data download to
        // the editor starts 3 to 4 seconds after turning the instrument on, so we wait 6 seconds
        // (from now) to give the download a chance to start before we begin monitoring for its
        // completion (see `on_message`).
        *self.download_status.lock().unwrap() = DownloadStatus::Waiting;
        *self.download_wait_start_time.lock().unwrap() = Some(Instant::now());
        if let Some(listener) = self.listener() {
            rayon::spawn(move || listener.on_receiving_data_started());
        }
    }

    fn on_receiving_data_stopped(&self) {
        if let Some(listener) = self.listener() {
            rayon::spawn(move || listener.on_receiving_data_stopped());
        }
    }

    fn on_devices_connected_changed(&self) {
        if let Some(listener) = self.listener() {
            rayon::spawn(move || listener.on_devices_connected_changed());
        }
    }
}
