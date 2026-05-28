use std::cell::RefCell;
use pitchgrid_continuum::i_tuner::ITuner;
use pitchgrid_continuum::midi_sender::IMidiSender;
use pitchgrid_continuum::tuning_params::FormattedTuningParams;
use pitchgrid_continuum::tuning_params::TuningParams;

/// Returns a clone of the current `TunerState`.
pub fn tuner_state() -> TunerState {
    TUNER_STATE.with(|s| s.borrow().clone())
}

pub struct MockTuner {}

impl MockTuner {
    pub fn new() -> Self {
        TUNER_STATE.replace(TunerState::new());
        MockTuner {}
    }
}

impl ITuner for MockTuner {
    fn init(&self, pitch_table: u8) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.init_count += 1;
            s.pitch_table = Some(pitch_table);
        });
    }

    fn on_tuning_received(&self, params: TuningParams) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.formatted_tuning_params = params.format_tuning_params();
            s.tuning_params = Some(params);
        });
    }

    fn has_data(&self) -> bool {
        TUNER_STATE.with_borrow_mut(|s| {
            s.has_data_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().has_data_result)
    }

    fn remove_data(&self) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.remove_data_count += 1;
        });
    }

    fn send_current_preset_update(&self) -> bool {
        TUNER_STATE.with_borrow_mut(|s| {
            s.send_current_preset_update_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().send_current_preset_update_result)
    }

    fn formatted_tuning_params(&self) -> FormattedTuningParams {
        TUNER_STATE.with(|s| s.borrow().formatted_tuning_params.clone())
    }

    fn is_root_freq_overridden(&self) -> bool {
        TUNER_STATE.with(|s| s.borrow().is_root_freq_overridden)
    }

    fn set_root_freq_override_note_no(&self, index: usize, send_tuning: bool) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.root_freq_override_note_no = Some(index);
            s.set_root_freq_override_note_no_send_tuning = Some(send_tuning);
            s.is_root_freq_overridden = index != 0;
        });
    }

    fn set_override_rounding_initial(&self, value: bool) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.override_rounding_initial = Some(value);
        });
    }

    fn set_override_rounding_rate(&self, value: bool) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.override_rounding_rate = Some(value);
        });
    }

    fn set_rounding_rate(&self, rate: u8) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.rounding_rate = Some(rate);
        });
    }

    fn set_pitch_table(&self, pitch_table: u8) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_pitch_table_count += 1;
            s.pitch_table = Some(pitch_table);
        });
    }

    fn on_tuning_updated(&self) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.on_tuning_updated_count += 1;
        });
    }

    fn set_midi_sender(&self, _sender: Box<dyn IMidiSender>) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_midi_sender_count += 1;
        });
    }

    fn pitch_table_index(&self) -> usize {
        TUNER_STATE.with(|s| s.borrow().pitch_table_index)
    }
}

#[derive(Clone)]
pub struct TunerState {
    pub pitch_table: Option<u8>,
    pub init_count: u16,

    pub tuning_params: Option<TuningParams>,

    pub has_data_count: u16,
    pub has_data_result: bool,

    pub remove_data_count: u16,

    pub send_current_preset_update_count: u16,
    pub send_current_preset_update_result: bool,

    pub formatted_tuning_params: FormattedTuningParams,

    pub is_root_freq_overridden: bool,

    pub root_freq_override_note_no: Option<usize>,
    pub set_root_freq_override_note_no_send_tuning: Option<bool>,

    pub override_rounding_initial: Option<bool>,
    pub override_rounding_rate: Option<bool>,
    pub rounding_rate: Option<u8>,
    pub set_pitch_table_count: u16,
    pub on_tuning_updated_count: u16,
    pub set_midi_sender_count: u16,
    pub pitch_table_index: usize,
}

impl TunerState {
    pub fn new() -> Self {
        TunerState {
            pitch_table: None,
            init_count: 0,

            tuning_params: None,

            has_data_count: 0,
            has_data_result: false,

            remove_data_count: 0,

            send_current_preset_update_count: 0,
            send_current_preset_update_result: false,

            formatted_tuning_params: FormattedTuningParams::default(),

            is_root_freq_overridden: false,

            root_freq_override_note_no: None,
            set_root_freq_override_note_no_send_tuning: None,
            override_rounding_initial: None,
            override_rounding_rate: None,
            rounding_rate: None,
            set_pitch_table_count: 0,
            on_tuning_updated_count: 0,
            set_midi_sender_count: 0,
            pitch_table_index: 0,
        }
    }
}

thread_local! {
    static TUNER_STATE: RefCell<TunerState> = RefCell::new(TunerState::new());
}
