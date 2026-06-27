use std::sync::Weak;

/// The semantic seam by which the `ContinuumProtocol` reports interpreted Continuum events to the
/// application (the `Presenter`). These are the events that were formerly raised directly by the
/// MIDI layer. Identical to the former `MidiCallbacks`, renamed to reflect that it is the
/// protocol-to-app boundary, not the raw MIDI one.
pub trait ContinuumProtocolListener: Send + Sync {
    fn on_download_completed(&self);
    fn on_download_started(&self);
    fn on_new_preset_selected(&self);
    fn on_devices_connected_changed(&self);
    fn on_receiving_data_started(&self);
    fn on_receiving_data_stopped(&self);
    fn on_tuning_updated(&self);
    fn on_updating_tuning(&self);
}

/// The `Presenter`'s handle on the protocol layer: query the download state and register the
/// listener. Behind a trait so a mock can be injected in tests, mirroring `IMidiManager`.
pub trait IContinuumProtocol: TuningUpdateSignaller {
    fn has_downloaded_init_data(&self) -> bool;

    /// Records the (weak) semantic listener the protocol raises events to. Weak to avoid a
    /// reference cycle, mirroring the `Presenter`'s `presenter_weak`. Set by `Presenter::init`.
    fn set_listener(&self, listener: Weak<dyn ContinuumProtocolListener>);
}

/// The seam by which the `Tuner` tells the protocol layer that it is about to send a tuning update,
/// so the layer can mark a tuning as in-flight (and notify the UI).
pub trait TuningUpdateSignaller: Send + Sync {
    fn on_updating_tuning(&self);
}

/// A no-op `TuningUpdateSignaller`, the `Tuner`'s default until the real one is wired in (see
/// `Presenter::new`). Mirrors `NullMidiSender`; keeps the standalone `Tuner` tests free of any
/// MIDI/protocol wiring.
pub struct NullTuningSignaller;

impl TuningUpdateSignaller for NullTuningSignaller {
    fn on_updating_tuning(&self) {}
}
