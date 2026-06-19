use std::sync::{Arc, LazyLock, Mutex, Weak};
use pitchgrid_continuum::i_continuum_protocol::{ContinuumProtocolListener, IContinuumProtocol};

pub static MOCK_CONTINUUM_PROTOCOL: LazyLock<Mutex<MockContinuumProtocol>> =
    LazyLock::new(|| Mutex::new(MockContinuumProtocol::new_state()));

/// Mock for `IContinuumProtocol`. It also stands in as the source of the semantic events the real
/// `ContinuumProtocol` would raise: `Controller::init` registers the controller via `set_listener`,
/// and the `simulate_*` helpers fire the listener directly — the role formerly played by
/// `MockMidiManager` (which captured the callbacks via `init`).
pub struct MockContinuumProtocol {
    listener: Option<Weak<dyn ContinuumProtocolListener>>,
    has_downloaded_init_data_result: bool,
}

impl MockContinuumProtocol {
    fn new_state() -> Self {
        MockContinuumProtocol {
            listener: None,
            has_downloaded_init_data_result: false,
        }
    }

    pub fn new() -> Arc<dyn IContinuumProtocol> {
        *MOCK_CONTINUUM_PROTOCOL.lock().unwrap_or_else(|e| e.into_inner()) =
            MockContinuumProtocol::new_state();
        Arc::new(MockContinuumProtocolImpl)
    }

    /// The registered semantic listener (the `Controller`). Clones the weak and releases the mock
    /// lock BEFORE upgrading and returning, so a callback that re-locks the mock cannot deadlock —
    /// the same discipline the `MockMidiManager` callbacks relied on.
    fn listener() -> Arc<dyn ContinuumProtocolListener> {
        let listener = MOCK_CONTINUUM_PROTOCOL.lock().unwrap_or_else(|e| e.into_inner())
            .listener.clone().expect("set_listener (via Controller::init) must run first");
        listener.upgrade().expect("the Controller must still be alive")
    }

    pub fn simulate_download_completed() {
        MOCK_CONTINUUM_PROTOCOL.lock().unwrap_or_else(|e| e.into_inner())
            .has_downloaded_init_data_result = true;
        Self::listener().on_download_completed();
    }

    pub fn simulate_download_started() {
        Self::listener().on_download_started();
    }

    pub fn simulate_devices_connected_changed() {
        Self::listener().on_devices_connected_changed();
    }

    pub fn simulate_new_preset_selected() {
        Self::listener().on_new_preset_selected();
    }

    pub fn simulate_receiving_data_started() {
        Self::listener().on_receiving_data_started();
    }

    pub fn simulate_receiving_data_stopped() {
        Self::listener().on_receiving_data_stopped();
    }

    pub fn simulate_tuning_updated() {
        Self::listener().on_tuning_updated();
    }

    pub fn simulate_updating_tuning() {
        Self::listener().on_updating_tuning();
    }
}

struct MockContinuumProtocolImpl;

impl IContinuumProtocol for MockContinuumProtocolImpl {
    fn has_downloaded_init_data(&self) -> bool {
        MOCK_CONTINUUM_PROTOCOL.lock().unwrap_or_else(|e| e.into_inner())
            .has_downloaded_init_data_result
    }

    fn set_listener(&self, listener: Weak<dyn ContinuumProtocolListener>) {
        MOCK_CONTINUUM_PROTOCOL.lock().unwrap_or_else(|e| e.into_inner()).listener = Some(listener);
    }
}
