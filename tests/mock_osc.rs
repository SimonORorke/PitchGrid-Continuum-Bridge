use std::cell::RefCell;
use std::sync::Arc;
use pitchgrid_continuum::i_osc::IOsc;
use pitchgrid_continuum::osc::OscCallbacks;

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
}

impl IOsc for MockOsc {
    #[allow(dead_code)]
    fn set_listening_port(&mut self, listening_port: u16) {
        OSC_STATE.with_borrow_mut(|s| {
            s.set_listening_port_count += 1;
            s.last_set_listening_port_listening_port = Some(listening_port);
        });
    }

    #[allow(dead_code)]
    fn start(&mut self, callbacks: Arc<dyn OscCallbacks>) {
        OSC_STATE.with_borrow_mut(|s| {
            s.start_count += 1;
            s.last_start_callbacks = Some(callbacks);
        });
    }

    #[allow(dead_code)]
    fn stop(&self) {
        OSC_STATE.with_borrow_mut(|s| {
            s.stop_count += 1;
        });
    }

    #[allow(dead_code)]
    fn is_pitchgrid_connected(&self) -> bool {
        OSC_STATE.with_borrow_mut(|s| {
            s.is_pitchgrid_connected_count += 1;
        });
        OSC_STATE.with(|s| s.borrow().is_pitchgrid_connected_result)
    }

    #[allow(dead_code)]
    fn is_running(&self) -> bool {
        OSC_STATE.with_borrow_mut(|s| {
            s.is_running_count += 1;
        });
        OSC_STATE.with(|s| s.borrow().is_running_result)
    }
}

pub struct OscState {
    pub set_listening_port_count: u16,
    pub last_set_listening_port_listening_port: Option<u16>,

    pub start_count: u16,
    pub last_start_callbacks: Option<Arc<dyn OscCallbacks>>,

    pub stop_count: u16,

    pub is_pitchgrid_connected_count: u16,
    pub is_pitchgrid_connected_result: bool,

    pub is_running_count: u16,
    pub is_running_result: bool,
}

impl OscState {
    pub fn new() -> Self {
        OscState {
            set_listening_port_count: 0,
            last_set_listening_port_listening_port: None,

            start_count: 0,
            last_start_callbacks: None,

            stop_count: 0,

            is_pitchgrid_connected_count: 0,
            is_pitchgrid_connected_result: false,

            is_running_count: 0,
            is_running_result: false,
        }
    }
}

impl Clone for OscState {
    fn clone(&self) -> Self {
        OscState {
            set_listening_port_count: self.set_listening_port_count,
            last_set_listening_port_listening_port: self.last_set_listening_port_listening_port,

            start_count: self.start_count,
            last_start_callbacks: self.last_start_callbacks.clone(),

            stop_count: self.stop_count,

            is_pitchgrid_connected_count: self.is_pitchgrid_connected_count,
            is_pitchgrid_connected_result: self.is_pitchgrid_connected_result,

            is_running_count: self.is_running_count,
            is_running_result: self.is_running_result,
        }
    }
}

thread_local! {
    static OSC_STATE: RefCell<OscState> = RefCell::new(OscState::new());
}
