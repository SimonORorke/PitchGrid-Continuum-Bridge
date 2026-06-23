use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use log::{debug, trace};
use crate::error_notifier::{SharedErrorNotifier};
use crate::i_continuum_protocol::{TuningUpdateSignaller, NullTuningSignaller};
use crate::midi_sender::{IMidiSender, NullMidiSender};
use crate::tuning_params::{FormattedTuningParams, TuningParams};

/// A facility for tuning a Continuum from PitchGrid parameters.
pub struct Tuner {
    is_already_updating: AtomicBool,
    is_another_update_pending: AtomicBool,
    override_rounding_initial: AtomicBool,
    override_rounding_rate: AtomicBool,
    rounding_rate: AtomicU8,
    root_freq_override_note_no: AtomicUsize,
    keys: Mutex<Vec<Key>>,
    midi_sender: Mutex<Box<dyn IMidiSender>>,
    /// The seam to the protocol layer, told when a tuning send begins (see `send_tuning_update`).
    /// Defaults to a no-op so a standalone `Tuner` (e.g. in `tuner_tests`) needs no wiring; the real
    /// one is injected by `Presenter::new`.
    tuning_signaller: Mutex<Arc<dyn TuningUpdateSignaller>>,
    params: Arc<Mutex<TuningParams>>,
}

impl Tuner {
    pub fn new() -> Self {
        Self {
            is_already_updating: AtomicBool::new(false),
            is_another_update_pending: AtomicBool::new(false),
            override_rounding_initial: AtomicBool::new(false),
            override_rounding_rate: AtomicBool::new(false),
            rounding_rate: AtomicU8::new(127),
            root_freq_override_note_no: AtomicUsize::new(0),
            keys: Mutex::new(vec![]),
            // Will be replaced by the usable MIDI sender in `set_midi_sender`.
            midi_sender: Mutex::new(Box::new(NullMidiSender)),
            tuning_signaller: Mutex::new(Arc::new(NullTuningSignaller)),
            params: Arc::new(Mutex::new(TuningParams::default())),
        }
    }

    pub fn init(&self, pitch_table: u8) {
        PITCH_TABLE.store(pitch_table, Ordering::Relaxed);
    }

    /// Update tuning parameters from the OSC message.
    ///     Args:
    ///         mode: Mode index
    ///         root_freq: Root pitch in Hz
    ///         stretch: Equave as log2 frequency ratio (e.g. 1.0 for octave)
    ///         skew: Generator as log2 frequency ratio
    ///         mode_offset: Mode offset (float)
    ///         steps: Number of steps per period
    ///         mos_a: MOS parameter a
    ///         mos_b: MOS parameter b
    ///             MOS (a,b) is not the same as (L,s). Sometimes (a,b) = (L,s) and sometimes
    ///             (a,b) = (s,L) depending on the relative size of the vectors.
    ///             The scalatrix MOS class also has mos.nL and mos.nS properties
    ///             which will be used when displaying the MOS system (like 5L 2s).
    pub fn on_tuning_received(&self, params: TuningParams) {
        debug!(
            "on_tuning_received: mode = {}; root_freq = {}; stretch = {}; \
            skew = {}; mode_offset = {}; steps = {}; mos_a = {}; mos_b = {}",
            params.mode(), params.root_freq(), params.stretch(),
            params.skew(), params.mode_offset(), params.steps(), params.mos_a(), params.mos_b());
        *self.params.lock().unwrap() = params;
        self.tune();
    }

    pub fn has_data(&self) -> bool {
        !self.keys.lock().unwrap().is_empty()
    }

    pub fn remove_data(&self) {
        trace!("remove_data: Setting is_already_updating to false");
        self.is_already_updating.store(false, Ordering::Relaxed);
        self.is_another_update_pending.store(false, Ordering::Relaxed);
        *self.params.lock().unwrap() = TuningParams::default();
        *self.keys.lock().unwrap() = vec![];
    }

    /// If a tuning table generated from tuning parameters received from PitchGrid has previously
    /// been sent to the instrument, sends a tuning update for the instrument's current preset.
    /// The current tuning table will be assigned to the preset, which will also be updated
    /// with any rounding parameters that have been specified.
    /// Returns whether an update has been sent.
    pub fn send_current_preset_update(&self) -> bool {
        debug!("send_current_preset_update");
        let can_update = self.has_data();
        if can_update {
            debug!("send_current_preset_update: Sending update");
            self.send_tuning_update(false);
        }
        can_update
    }

    /// The tuning parameters formatted for display.
    pub fn formatted_tuning_params(&self) -> FormattedTuningParams {
        if !self.has_data() {
            return FormattedTuningParams::default();
        }
        let params = self.params.lock().unwrap();
        params.format_tuning_params()
    }

    pub fn is_root_freq_overridden(&self) -> bool {
        self.root_freq_override_note_no.load(Ordering::Relaxed) != 0
    }

    /// Sets the root frequency override and optionally sends it to the instrument.
    pub fn set_root_freq_override_note_no(&self, index: usize, send_tuning: bool) {
        let note_no = if index == 0 {
            0usize // No override
        } else {
            index + 53 // E.g. for middle C, index = 7, note_no = 60.
        };
        self.root_freq_override_note_no.store(note_no, Ordering::Relaxed);
        if send_tuning && self.has_data() {
            self.tune();
        }
    }

    /// Sets whether initial rounding is to be overridden the next time tuning is sent.
    pub fn set_override_rounding_initial(&self, value: bool) {
        self.override_rounding_initial.store(value, Ordering::Relaxed);
    }

    /// Sets whether rounding rate is to be overridden the next time tuning is sent.
    pub fn set_override_rounding_rate(&self, value: bool) {
        self.override_rounding_rate.store(value, Ordering::Relaxed);
    }

    /// Sets the rounding rate, if it is to be overridden, the next time tuning is sent.
    pub fn set_rounding_rate(&self, rate: u8) {
        self.rounding_rate.store(rate, Ordering::Relaxed);
    }

    pub fn set_pitch_table(&self, pitch_table: u8) {
        PITCH_TABLE.store(pitch_table, Ordering::Relaxed);
    }

    pub fn on_tuning_updated(&self) {
        debug!("on_tuning_updated");
        if !self.has_data() {
            debug!("on_tuning_updated: no tuning data");
            // Could be tuning updated when an instrument preset is loaded
            // while PitchGrid is not connected.
            return;
        }
        // See comment in on_tuning_received.
        let send_again: bool;
        {
            let is_another_update_pending = self.is_another_update_pending.load(Ordering::Relaxed);
            trace!("on_tuning_updated: is_another_update_pending = {is_another_update_pending}");
            if is_another_update_pending {
                self.is_another_update_pending.store(false, Ordering::Relaxed);
                send_again = true;
            } else {
                trace!("on_tuning_updated: Setting is_already_updating to false");
                self.is_already_updating.store(false, Ordering::Relaxed);
                send_again = false;
            }
        }
        if send_again {
            self.send_tuning_update(true);
        }
    }

    /// Sets the MIDI sender: the real one (wired by `Presenter::new`) in production, a mock in tests.
    pub fn set_midi_sender(&self, sender: Box<dyn IMidiSender>) {
        *self.midi_sender.lock().unwrap() = sender;
    }

    /// Sets the tuning-update signaller, wired by `Presenter::new` to the shared MIDI state.
    pub fn set_tuning_signaller(&self, signaller: Arc<dyn TuningUpdateSignaller>) {
        *self.tuning_signaller.lock().unwrap() = signaller;
    }

    pub fn pitch_table_index(&self) -> usize {
        // Return the index of the pitch_tables item that equals pitch_table.
        Self::pitch_tables().iter().position(|&x| x == Self::pitch_table()).unwrap_or(0)
    }

    /// Returns the currently selected pitch table number.
    /// Remains an associated function so that midi.rs can call it without holding a tuner reference.
    pub fn pitch_table() -> u8 {
        PITCH_TABLE.load(Ordering::Relaxed)
    }

    pub fn pitch_tables<'a>() -> &'a Vec<u8> {
        PITCH_TABLES.get_or_init(|| (80..88).collect())
    }

    pub fn default_pitch_table() -> u8 { 80 }

    pub fn midi_send_error_notifier(&self) -> SharedErrorNotifier {
        self.midi_sender.lock().unwrap().error_notifier()
    }

    /// Calculates the tuning and either sends it to the instrument, provided another tuning update
    /// is not already in progress, or stores it for sending once the current update completes.
    fn tune(&self) {
        debug!("tune");
        let send_now: bool;
        {
            let mut params = self.params.lock().unwrap();
            let key_pitches = params.calculate_key_pitches(
                self.root_freq_override_note_no.load(Ordering::Relaxed));
            *self.keys.lock().unwrap() = key_pitches.iter().enumerate()
                .map(|(i, pitch)| Key {
                    number: i as u8,
                    required_pitch: *pitch,
                    to_number: 0,
                    offset_ratio: 0.0,
                    offset_msb: 0,
                    offset_lsb: 0,
                }).collect();
            // If the player sweeps one of the tuning controls in PitchGrid,
            // we will receive new tunings much faster than the instrument can update the tuning
            // table, which takes in the order of half a second.
            // If we were to keep sending tunings to the instrument regardless, its processor would
            // be swamped, probably for minutes.
            // The solution is to not send more updates to the instrument while another update is
            // in progress and, once the update is complete, send the most recently received
            // following tuning if there is one.
            let is_already_updating = self.is_already_updating.load(Ordering::Relaxed);
            trace!("tune: is_already_updating = {is_already_updating}");
            if is_already_updating {
                self.is_another_update_pending.store(true, Ordering::Relaxed);
                send_now = false;
            } else {
                trace!("tune: Setting is_already_updating to true");
                self.is_already_updating.store(true, Ordering::Relaxed);
                send_now = true;
            }
        }
        if send_now {
            self.send_tuning_update(true);
        }
    }

    /// Sends the instrument a tuning update.
    ///
    /// generate: Whether a tuning table is to be generated from the tuning parameters received
    /// from PitchGrid and sent to the instrument. Set to false if the latest tuning table has
    /// previously been sent to the instrument.
    ///
    /// The tuning table will be assigned to the instrument's current preset, which will also be
    /// updated with any rounding parameters that have been specified.
    fn send_tuning_update(&self, generate: bool) {
        debug!("send_tuning_update: generate = {}", generate);
        self.tuning_signaller.lock().unwrap().on_updating_tuning();
        if generate {
            let mut keys = self.keys.lock().unwrap().clone();
            self.set_to_key_numbers(&mut keys);
            self.calculate_offsets(&mut keys);
            self.send_pitch_table(Self::pitch_table(), &keys);
        }
        // The following commands update the instrument's current preset.
        self.send_rounding_params();
        // Set active pitch table for performance.
        debug!("send_tuning_update: Setting active pitch table to {}", Self::pitch_table());
        self.midi_sender.lock().unwrap().send_control_change(16, 51, Self::pitch_table());
    }

    /// Sets the to_number field of each Key to the index of the pitch in DEFAULT_KEY_PITCHES
    /// that matches the Key's pitch.
    fn set_to_key_numbers(&self, keys: &mut [Key]) {
        for key_being_matched in keys.iter_mut() {
            key_being_matched.to_number = TuningParams::default_pitch_keys()
                .binary_search_by(|&x| x.partial_cmp(&key_being_matched.required_pitch).unwrap())
                .unwrap_or_else(|i| {
                    if i == 0 { 0 } else { i - 1 }
                }) as u8;
        }
    }

    /// Calculates the offset ratio and 14-bit offset values for each note.
    fn calculate_offsets(&self, keys: &mut [Key]) {
        for key in keys.iter_mut() {
            let note_pitch = key.required_pitch;
            let to_note_pitch = TuningParams::default_pitch_keys()[key.to_number as usize];
            let offset_ratio = (12.0 * (note_pitch / to_note_pitch).log2()).clamp(0.0, 1.0);
            key.offset_ratio = offset_ratio;
            let offset_14bit = (offset_ratio * 16383.0).round() as u16;
            key.offset_msb = ((offset_14bit >> 7) & 0x7F) as u8;
            key.offset_lsb = (offset_14bit & 0x7F) as u8;
        }
    }

    fn send_pitch_table(&self, pitch_table: u8, keys: &Vec<Key>) {
        let mut sender = self.midi_sender.lock().unwrap();
        // Select pitch table to update.
        sender.send_control_change(16, 109, pitch_table);
        // Tuning for each MIDI key
        for key in keys {
            // Base/From MIDI key
            sender.send_control_change(16, 38, key.number);
            // Base/From MIDI key tuning MSB
            sender.send_control_change(16, 38, 0);
            // Base/From MIDI key tuning LSB
            sender.send_control_change(16, 38, 0);
            // Re-tuned/To MIDI key
            sender.send_control_change(16, 38, key.to_number);
            // Re-tuned/To MIDI key tuning MSB
            sender.send_control_change(16, 38, key.offset_msb);
            // Re-tuned/To MIDI key tuning LSB
            sender.send_control_change(16, 38, key.offset_lsb);
        }
        // Save pitch table on instrument.
        sender.send_control_change(16, 109, 101);
    }

    /// Sends pitch rounding parameters, if required, to the instrument.
    fn send_rounding_params(&self) {
        let mut sender = self.midi_sender.lock().unwrap();
        if self.override_rounding_initial.load(Ordering::Relaxed) {
            // Turn on Rounding Initial
            debug!("send_rounding_params: Sending Rounding Initial");
            sender.send_control_change(1, 28, 127); // RndIni
        }
        if self.override_rounding_rate.load(Ordering::Relaxed) {
            // Rounding Mode Normal
            debug!("send_rounding_params: Sending Rounding Mode Normal");
            sender.send_matrix_poke(10, 0); // RoundMode
            // Rounding Rate
            debug!("send_rounding_params: Sending Rounding Rate");
            sender.send_control_change(
                1, 25, self.rounding_rate.load(Ordering::Relaxed)); // RoundRate
        }
    }
}

#[derive(Clone, Debug)]
struct Key {
    /// The MIDI note number of the key (0-127).
    number: u8,
    /// The pitch required for the note, in Hz.
    required_pitch: f32,
    /// The MIDI note number of the key with the closest standard tuning pitch
    /// that is less than or equal to the required pitch.
    to_number: u8,
    /// The offset ratio (0.0 to 1.0) as a fraction of a semitone, between the required pitch
    /// and the closest standard tuning pitch that is less than or equal to it.
    offset_ratio: f32,
    /// The upper 7 bits (MSB) of the offset ratio, as a 14-bit value,
    /// for sending to the instrument via MIDI.
    offset_msb: u8,
    /// The lower 7 bits (LSB) of the offset ratio, as a 14-bit value,
    /// for sending to the instrument via MIDI.
    offset_lsb: u8,
}

/// About the name Pitch Table.
/// The EaganMatrix Overlay Developer's Guide calls them tuning grids.
/// But naming is inconsistent in the Continuum User Guide.
/// The main documentation calls them pitch tables or custom grids, though there are also
/// scattered references to tuning grids.
/// We call them pitch tables, as that has the best chance of being understood by users.
///
/// Remains a static so that Tuner::pitch_table() can be called without holding a tuner reference.
static PITCH_TABLE: AtomicU8 = AtomicU8::new(0);

static PITCH_TABLES: OnceLock<Vec<u8>> = OnceLock::new();

pub type SharedTuner = Arc<Tuner>;
