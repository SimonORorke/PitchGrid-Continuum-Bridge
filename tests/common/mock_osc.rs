use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use pitchgrid_continuum::i_osc::{IOsc, OscCallbacks};
use pitchgrid_continuum::tuning_params::TuningParams;

pub fn mock_osc() -> MutexGuard<'static, MockOsc> {
    MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner())
}

pub static MOCK_OSC: LazyLock<Mutex<MockOsc>> =
    LazyLock::new(|| Mutex::new(MockOsc::new_state()));

pub struct MockOsc {
    pub set_listening_port_count: u16,
    pub listening_port: Option<u16>,

    pub start_count: u16,
    pub callbacks: Option<Arc<dyn OscCallbacks>>,

    pub stop_count: u16,

    pub is_pitchgrid_connected_count: u16,
    pub is_pitchgrid_connected_result: bool,

    pub is_running_count: u16,
    pub is_running_result: bool,

    pub tuning_params: Option<TuningParams>,
}

impl MockOsc {
    fn new_state() -> Self {
        MockOsc {
            set_listening_port_count: 0,
            listening_port: None,

            start_count: 0,
            callbacks: None,

            stop_count: 0,

            is_pitchgrid_connected_count: 0,
            is_pitchgrid_connected_result: false,

            is_running_count: 0,
            is_running_result: false,

            tuning_params: None,
        }
    }

    pub fn new() -> Self {
        *MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner()) = MockOsc::new_state();
        MockOsc::new_state()
    }

    pub fn set_is_running_result(value: bool) {
        MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner()).is_running_result = value;
    }

    pub fn simulate_pitchgrid_connected_changed(is_pitchgrid_connected: bool) {
        let callbacks = {
            let mut state = MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner());
            state.is_pitchgrid_connected_result = is_pitchgrid_connected;
            state.callbacks.clone().unwrap()
        };
        callbacks.on_pitchgrid_connected_changed();
    }

    pub fn simulate_tuning_received(tuning_params: TuningParams) {
        let callbacks = {
            let mut state = MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner());
            state.tuning_params = Some(tuning_params.clone());
            state.callbacks.clone().unwrap()
        };
        callbacks.on_tuning_received(tuning_params);
    }
}

impl IOsc for MockOsc {
    fn set_listening_port(&mut self, listening_port: u16) {
        let mut state = MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner());
        state.set_listening_port_count += 1;
        state.listening_port = Some(listening_port);
    }

    fn start(&mut self, callbacks: Arc<dyn OscCallbacks>) {
        let mut state = MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner());
        state.start_count += 1;
        state.callbacks = Some(callbacks);
    }

    fn stop(&self) {
        MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner()).stop_count += 1;
    }

    fn is_pitchgrid_connected(&self) -> bool {
        let mut state = MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner());
        state.is_pitchgrid_connected_count += 1;
        state.is_pitchgrid_connected_result
    }

    fn is_running(&self) -> bool {
        let mut state = MOCK_OSC.lock().unwrap_or_else(|e| e.into_inner());
        state.is_running_count += 1;
        state.is_running_result
    }
}
