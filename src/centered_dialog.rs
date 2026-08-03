use slint::{ComponentHandle, PlatformError};
use crate::{AboutWindow, NewVersionWindow};

/// A dialog box that is to be shown centred over the main window.
pub trait CenteredDialog {
    fn show(&self) -> Result<(), PlatformError>;
    fn window(&self) -> &slint::Window;
    fn get_preferred_width(&self) -> f32;
    fn get_preferred_height(&self) -> f32;
}

impl CenteredDialog for AboutWindow {
    fn show(&self) -> Result<(), PlatformError> {
        ComponentHandle::show(self)
    }
    fn window(&self) -> &slint::Window {
        ComponentHandle::window(self)
    }
    fn get_preferred_width(&self) -> f32 {
        self.get_preferred_w()
    }
    fn get_preferred_height(&self) -> f32 {
        self.get_preferred_h()
    }
}

impl CenteredDialog for NewVersionWindow {
    fn show(&self) -> Result<(), PlatformError> {
        ComponentHandle::show(self)
    }
    fn window(&self) -> &slint::Window {
        ComponentHandle::window(self)
    }
    fn get_preferred_width(&self) -> f32 {
        self.get_preferred_w()
    }
    fn get_preferred_height(&self) -> f32 {
        self.get_preferred_h()
    }
}
