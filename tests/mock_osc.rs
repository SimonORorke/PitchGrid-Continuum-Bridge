use std::cell::RefCell;
use std::sync::Arc;
use pitchgrid_continuum::i_osc::IOsc;
use pitchgrid_continuum::i_osc::OscCallbacks;
use pitchgrid_continuum::tuning_params::TuningParams;

/// Returns a clone of the current `OscState`.
pub fn osc_state() -> OscState {
    OSC_STATE.with(|s| s.borrow().clone())
}

pub struct MockOsc {}

impl MockOsc {
    pub fn new() -> Self {
        OSC_STATE.replace(OscState::new());
        MockOsc {}
    }

    pub fn set_is_running_result(value: bool) {
        OSC_STATE.with_borrow_mut(|s| {
            s.is_running_result = value;
        });
    }

    pub fn simulate_pitchgrid_connected_changed(is_pitchgrid_connected: bool) {
        let callbacks = OSC_STATE.with_borrow_mut(|s| {
            s.is_pitchgrid_connected_result = is_pitchgrid_connected;
            s.callbacks.clone().unwrap()
        });
        callbacks.on_pitchgrid_connected_changed();
    }

    pub fn simulate_tuning_received(tuning_params: TuningParams) {
        let callbacks = OSC_STATE.with_borrow_mut(|s| {
            s.tuning_params = Some(tuning_params.clone());
            s.callbacks.clone().unwrap()
        });
        callbacks.on_tuning_received(tuning_params);
    }
}


impl IOsc for MockOsc {
    fn set_listening_port(&mut self, listening_port: u16) {
        OSC_STATE.with_borrow_mut(|s| {
            s.set_listening_port_count += 1;
            s.listening_port = Some(listening_port);
        });
    }

    fn start(&mut self, callbacks: Arc<dyn OscCallbacks>) {
        OSC_STATE.with_borrow_mut(|s| {
            s.start_count += 1;
            s.callbacks = Some(callbacks);
        });
    }

    fn stop(&self) {
        OSC_STATE.with_borrow_mut(|s| {
            s.stop_count += 1;
        });
    }

    fn is_pitchgrid_connected(&self) -> bool {
        OSC_STATE.with_borrow_mut(|s| {
            s.is_pitchgrid_connected_count += 1;
        });
        OSC_STATE.with(|s| s.borrow().is_pitchgrid_connected_result)
    }

    fn is_running(&self) -> bool {
        OSC_STATE.with_borrow_mut(|s| {
            s.is_running_count += 1;
        });
        OSC_STATE.with(|s| s.borrow().is_running_result)
    }
}

pub struct OscState {
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

impl OscState {
    pub fn new() -> Self {
        OscState {
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
}

impl Clone for OscState {
    fn clone(&self) -> Self {
        OscState {
            set_listening_port_count: self.set_listening_port_count,
            listening_port: self.listening_port,

            start_count: self.start_count,
            callbacks: self.callbacks.clone(),

            stop_count: self.stop_count,

            is_pitchgrid_connected_count: self.is_pitchgrid_connected_count,
            is_pitchgrid_connected_result: self.is_pitchgrid_connected_result,

            is_running_count: self.is_running_count,
            is_running_result: self.is_running_result,

            tuning_params: self.tuning_params.clone(),
        }
    }
}

thread_local! {
    static OSC_STATE: RefCell<OscState> = RefCell::new(OscState::new());
}
