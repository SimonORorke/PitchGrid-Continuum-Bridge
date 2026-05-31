use std::sync::Arc;
use crate::tuning_params::TuningParams;

pub trait OscCallbacks: Send + Sync {
    fn on_pitchgrid_connected_changed(&self);
    fn on_tuning_received(&self, tuning_params: TuningParams);
}

/// A trait that defines the interface for communicating with PitchGrid via OSC.
///
/// For the The `I` prefix, see `ITuner`s doc comment.
pub trait IOsc: Send + Sync {
    fn set_listening_port(&mut self, listening_port: u16);
    fn start(&mut self, callbacks: Arc<dyn OscCallbacks>);
    fn stop(&self);
    fn is_pitchgrid_connected(&self) -> bool;
    fn is_running(&self) -> bool;
}
