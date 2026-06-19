use std::sync::{LazyLock, Mutex, MutexGuard};
use pitchgrid_continuum::global::MessageType;
use pitchgrid_continuum::i_ui_methods::IUiMethods;
use pitchgrid_continuum::device_strategy::DeviceStrategy;
use pitchgrid_continuum::tuning_params::FormattedTuningParams;

pub fn mock_ui_methods() -> MutexGuard<'static, MockUiMethods> {
    MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner())
}

pub static MOCK_UI_METHODS: LazyLock<Mutex<MockUiMethods>> =
    LazyLock::new(|| Mutex::new(MockUiMethods::new_state()));

pub struct MockUiMethods {
    pub focus_device_count: u16,
    pub focus_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub get_selected_device_index_count: u16,
    pub get_selected_device_index_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub set_selected_device_index_count: u16,
    pub selected_device_index: Option<usize>,
    pub set_selected_device_index_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub set_devices_model_count: u16,
    pub set_devices_model_device_names: Option<Vec<String>>,
    pub set_devices_model_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub show_connected_device_name_count: u16,
    pub show_connected_device_name_name: Option<String>,
    pub show_connected_device_name_msg_type: Option<MessageType>,
    pub show_connected_device_name_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub show_message_count: u16,
    pub show_message_msg: Option<String>,
    pub show_message_msg_type: Option<MessageType>,

    pub show_pitchgrid_status_count: u16,
    pub show_pitchgrid_status_msg: Option<String>,
    pub show_pitchgrid_status_msg_type: Option<MessageType>,

    pub show_tuning_count: u16,
    pub show_tuning_formatted_tuning: Option<FormattedTuningParams>,
    pub show_tuning_is_root_freq_overridden: Option<bool>,

    pub main_window_position_x: Option<i32>,
    pub main_window_position_y: Option<i32>,

    pub override_rounding_initial: Option<bool>,
    pub override_rounding_rate: Option<bool>,
    pub rounding_rate: Option<u8>,
    pub selected_osc_listening_port_index: Option<i32>,
    pub selected_pitch_table_index: Option<i32>,
}

impl MockUiMethods {
    fn new_state() -> Self {
        MockUiMethods {
            focus_device_count: 0,
            focus_device_strategy: None,

            get_selected_device_index_count: 0,
            get_selected_device_index_device_strategy: None,

            set_selected_device_index_count: 0,
            selected_device_index: None,
            set_selected_device_index_device_strategy: None,

            set_devices_model_count: 0,
            set_devices_model_device_names: None,
            set_devices_model_device_strategy: None,

            show_connected_device_name_count: 0,
            show_connected_device_name_name: None,
            show_connected_device_name_msg_type: None,
            show_connected_device_name_device_strategy: None,

            show_message_count: 0,
            show_message_msg: None,
            show_message_msg_type: None,

            show_pitchgrid_status_count: 0,
            show_pitchgrid_status_msg: None,
            show_pitchgrid_status_msg_type: None,

            show_tuning_count: 0,
            show_tuning_formatted_tuning: None,
            show_tuning_is_root_freq_overridden: None,

            main_window_position_x: None,
            main_window_position_y: None,
            override_rounding_initial: None,
            override_rounding_rate: None,
            rounding_rate: None,
            selected_osc_listening_port_index: None,
            selected_pitch_table_index: None,
        }
    }

    pub fn new() -> Self {
        *MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner()) = MockUiMethods::new_state();
        MockUiMethods::new_state()
    }

    pub fn set_selected_device_index(index: usize) {
        MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner()).selected_device_index =
            Some(index);
    }
}

impl IUiMethods for MockUiMethods {
    fn focus_device(&self, device_strategy: &dyn DeviceStrategy) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.focus_device_count += 1;
        state.focus_device_strategy = Some(device_strategy.clone_box());
    }

    fn get_selected_device_index(&self, device_strategy: &dyn DeviceStrategy) -> usize {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.get_selected_device_index_count += 1;
        state.get_selected_device_index_device_strategy = Some(device_strategy.clone_box());
        state.selected_device_index.unwrap_or(0)
    }

    fn set_selected_device_index(&self, index: usize, device_strategy: &dyn DeviceStrategy) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.set_selected_device_index_count += 1;
        state.selected_device_index = Some(index);
        state.set_selected_device_index_device_strategy = Some(device_strategy.clone_box());
    }

    fn set_devices_model(&self, device_names: &[String], device_strategy: &dyn DeviceStrategy) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.set_devices_model_count += 1;
        state.set_devices_model_device_names = Some(device_names.to_vec());
        state.set_devices_model_device_strategy = Some(device_strategy.clone_box());
    }

    fn show_connected_device_name(&self, name: &str, msg_type: MessageType,
                                  device_strategy: &dyn DeviceStrategy) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.show_connected_device_name_count += 1;
        state.show_connected_device_name_name = Some(name.to_string());
        state.show_connected_device_name_msg_type = Some(msg_type);
        state.show_connected_device_name_device_strategy = Some(device_strategy.clone_box());
    }

    fn show_message(&self, msg: &str, msg_type: MessageType) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.show_message_count += 1;
        state.show_message_msg = Some(msg.to_string());
        state.show_message_msg_type = Some(msg_type);
    }

    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.show_pitchgrid_status_count += 1;
        state.show_pitchgrid_status_msg = Some(status.to_string());
        state.show_pitchgrid_status_msg_type = Some(msg_type);
    }

    fn show_tuning(&self, tuning: FormattedTuningParams, is_root_freq_overridden: bool) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.show_tuning_count += 1;
        state.show_tuning_formatted_tuning = Some(tuning);
        state.show_tuning_is_root_freq_overridden = Some(is_root_freq_overridden);
    }

    fn set_main_window_position(&self, x: i32, y: i32) {
        let mut state = MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner());
        state.main_window_position_x = Some(x);
        state.main_window_position_y = Some(y);
    }

    fn set_override_rounding_initial(&self, value: bool) {
        MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner()).override_rounding_initial =
            Some(value);
    }

    fn set_override_rounding_rate(&self, value: bool) {
        MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner()).override_rounding_rate =
            Some(value);
    }

    fn set_rounding_rate(&self, rate: u8) {
        MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner()).rounding_rate = Some(rate);
    }

    fn set_selected_osc_listening_port_index(&self, index: i32) {
        MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner())
            .selected_osc_listening_port_index = Some(index);
    }

    fn set_selected_pitch_table_index(&self, index: i32) {
        MOCK_UI_METHODS.lock().unwrap_or_else(|e| e.into_inner()).selected_pitch_table_index =
            Some(index);
    }
}
