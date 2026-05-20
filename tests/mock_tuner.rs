use std::cell::RefCell;
use pitchgrid_continuum::i_tuner::ITuner;
use pitchgrid_continuum::midi_sender::IMidiSender;
use pitchgrid_continuum::tuner::FormattedTuningParams;
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
    #[allow(dead_code)]
    fn init(&self, pitch_table: u8) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.init_count += 1;
            s.init_pitch_table = Some(pitch_table);
        });
    }

    #[allow(dead_code)]
    fn on_tuning_received(&self, params: TuningParams) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.on_tuning_received_count += 1;
            s.on_tuning_received_params = Some(params);
        });
    }

    #[allow(dead_code)]
    fn has_data(&self) -> bool {
        TUNER_STATE.with_borrow_mut(|s| {
            s.has_data_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().has_data_result)
    }

    #[allow(dead_code)]
    fn remove_data(&self) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.remove_data_count += 1;
        });
    }

    #[allow(dead_code)]
    fn send_current_preset_update(&self) -> bool {
        TUNER_STATE.with_borrow_mut(|s| {
            s.send_current_preset_update_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().send_current_preset_update_result)
    }

    #[allow(dead_code)]
    fn formatted_tuning_params(&self) -> FormattedTuningParams {
        TUNER_STATE.with_borrow_mut(|s| {
            s.formatted_tuning_params_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().formatted_tuning_params_result.clone())
    }

    #[allow(dead_code)]
    fn is_root_freq_overridden(&self) -> bool {
        TUNER_STATE.with_borrow_mut(|s| {
            s.is_root_freq_overridden_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().is_root_freq_overridden_result)
    }

    #[allow(dead_code)]
    fn set_root_freq_override_note_no(&self, index: usize, send_tuning: bool) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_root_freq_override_note_no_count += 1;
            s.set_root_freq_override_note_no_index = Some(index);
            s.set_root_freq_override_note_no_send_tuning = Some(send_tuning);
        });
    }

    #[allow(dead_code)]
    fn set_override_rounding_initial(&self, value: bool) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_override_rounding_initial_count += 1;
            s.set_override_rounding_initial_value = Some(value);
        });
    }

    #[allow(dead_code)]
    fn set_override_rounding_rate(&self, value: bool) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_override_rounding_rate_count += 1;
            s.set_override_rounding_rate_value = Some(value);
        });
    }

    #[allow(dead_code)]
    fn set_rounding_rate(&self, rate: u8) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_rounding_rate_count += 1;
            s.set_rounding_rate_rate = Some(rate);
        });
    }

    #[allow(dead_code)]
    fn set_pitch_table(&self, pitch_table: u8) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_pitch_table_count += 1;
            s.set_pitch_table_pitch_table = Some(pitch_table);
        });
    }

    #[allow(dead_code)]
    fn on_tuning_updated(&self) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.on_tuning_updated_count += 1;
        });
    }

    #[allow(dead_code)]
    fn set_midi_sender(&self, _sender: Box<dyn IMidiSender>) {
        TUNER_STATE.with_borrow_mut(|s| {
            s.set_midi_sender_count += 1;
        });
    }

    #[allow(dead_code)]
    fn pitch_table_index(&self) -> usize {
        TUNER_STATE.with_borrow_mut(|s| {
            s.pitch_table_index_count += 1;
        });
        TUNER_STATE.with(|s| s.borrow().pitch_table_index_result)
    }
}

#[derive(Clone)]
pub struct TunerState {
    pub init_count: u16,
    pub init_pitch_table: Option<u8>,

    pub on_tuning_received_count: u16,
    pub on_tuning_received_params: Option<TuningParams>,

    pub has_data_count: u16,
    pub has_data_result: bool,

    pub remove_data_count: u16,

    pub send_current_preset_update_count: u16,
    pub send_current_preset_update_result: bool,

    pub formatted_tuning_params_count: u16,
    pub formatted_tuning_params_result: FormattedTuningParams,

    pub is_root_freq_overridden_count: u16,
    pub is_root_freq_overridden_result: bool,

    pub set_root_freq_override_note_no_count: u16,
    pub set_root_freq_override_note_no_index: Option<usize>,
    pub set_root_freq_override_note_no_send_tuning: Option<bool>,

    pub set_override_rounding_initial_count: u16,
    pub set_override_rounding_initial_value: Option<bool>,

    pub set_override_rounding_rate_count: u16,
    pub set_override_rounding_rate_value: Option<bool>,

    pub set_rounding_rate_count: u16,
    pub set_rounding_rate_rate: Option<u8>,

    pub set_pitch_table_count: u16,
    pub set_pitch_table_pitch_table: Option<u8>,

    pub on_tuning_updated_count: u16,

    pub set_midi_sender_count: u16,

    pub pitch_table_index_count: u16,
    pub pitch_table_index_result: usize,
}

impl TunerState {
    pub fn new() -> Self {
        TunerState {
            init_count: 0,
            init_pitch_table: None,

            on_tuning_received_count: 0,
            on_tuning_received_params: None,

            has_data_count: 0,
            has_data_result: false,

            remove_data_count: 0,

            send_current_preset_update_count: 0,
            send_current_preset_update_result: false,

            formatted_tuning_params_count: 0,
            formatted_tuning_params_result: FormattedTuningParams::default(),

            is_root_freq_overridden_count: 0,
            is_root_freq_overridden_result: false,

            set_root_freq_override_note_no_count: 0,
            set_root_freq_override_note_no_index: None,
            set_root_freq_override_note_no_send_tuning: None,

            set_override_rounding_initial_count: 0,
            set_override_rounding_initial_value: None,

            set_override_rounding_rate_count: 0,
            set_override_rounding_rate_value: None,

            set_rounding_rate_count: 0,
            set_rounding_rate_rate: None,

            set_pitch_table_count: 0,
            set_pitch_table_pitch_table: None,

            on_tuning_updated_count: 0,

            set_midi_sender_count: 0,

            pitch_table_index_count: 0,
            pitch_table_index_result: 0,
        }
    }
}

thread_local! {
    static TUNER_STATE: RefCell<TunerState> = RefCell::new(TunerState::new());
}
