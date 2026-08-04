use std::rc::Rc;
use std::sync::{Arc, Mutex};
use log::trace;
use slint::{ComponentHandle, PhysicalPosition, Weak, WindowPosition};
use app_info::{APP_TITLE,};
use crate::{ComboBoxItem, ComboBoxModel as MainComboBoxModel, MainWindow, NewVersionWindow,
            SlintMessageType};
use crate::centered_dialog::CenteredDialog;
use crate::device_strategy::DeviceStrategy;
use crate::global::{MessageType, DeviceType};
use crate::i_ui_methods::IUiMethods;
use crate::presenter::NewVersionCallbacks;
use crate::tuning_params::FormattedTuningParams;

/// This struct contains the methods called by `Presenter` to make changes to the UI.
/// It is part of the view in the Model-View-Presenter (MVP) pattern.
/// See `Presenter`'s doc comment for more information on how the project implements MVP.
#[derive(Clone)]
pub struct UiMethods {
    main_window_weak: Weak<MainWindow>,
    new_version_window_weak: slint::Weak<NewVersionWindow>,
}

impl UiMethods {
    pub fn new(main_window_weak: Weak<MainWindow>,
               new_version_window_weak: Weak<NewVersionWindow>) -> Self {
        Self { main_window_weak, new_version_window_weak }
    }

    /// Shows the dialog in the centre of the main window.
    pub fn show_dialog_in_centre_of_main_window(&self, dialog: &impl CenteredDialog) {
        // We have to show the dialog first before we can position it.
        dialog.show().unwrap();
        if let Some(main_window) = self.main_window_weak.upgrade() {
            let scale_factor = main_window.window().scale_factor();

            // Explicitly set the window size to the preferred dimensions if required
            if dialog.should_force_size() {
                let dw = (dialog.get_preferred_width() * scale_factor) as u32;
                let dh = (dialog.get_preferred_height() * scale_factor) as u32;
                dialog.window().set_size(slint::PhysicalSize::new(dw, dh));
            }

            // Calculate dimensions for centering
            let dw_phys = (dialog.get_preferred_width() * scale_factor) as i32;
            let dh_phys = (dialog.get_preferred_height() * scale_factor) as i32;
            let ms = main_window.window().size(); // Physical
            let mp = main_window.window().position();

            // Parent dimensions are already physical
            let pw_phys = ms.width as i32;
            let ph_phys = ms.height as i32;

            let x = mp.x + (pw_phys - dw_phys) / 2;
            let y = mp.y + (ph_phys - dh_phys) / 2;

            dialog.window().set_position(WindowPosition::Physical(PhysicalPosition { x, y }));
        }
    }


    /// Provide the specified closure with a MainWindow instance without returning a result.
    /// This method supports invocation from both the UI event loop and non-UI threads.
    fn with_main_window<F>(&self, f: F)
    where
        F: FnOnce(&MainWindow) + Send + 'static,
    {
        let weak = self.main_window_weak.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(main_window) = weak.upgrade() {
                f(&main_window);
            }
        }).unwrap();
    }

    /// Provide the specified closure with a MainWindow instance and return its result.
    /// Blocks the calling thread until the closure completes on the UI event loop.
    /// Must be called from a non-UI thread to avoid deadlock.
    fn with_main_window_result<T, F>(&self, f: F) -> T
    where
        T: Send + Default + 'static,
        F: FnOnce(&MainWindow) -> T + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let weak = self.main_window_weak.clone();
        slint::invoke_from_event_loop(move || {
            let result = if let Some(main_window) = weak.upgrade() {
                f(&main_window)
            } else {
                T::default()
            };
            tx.send(result).ok();
        }).unwrap();
        rx.recv().unwrap_or_default()
    }

    /// Provide the specified closure for execution on the UI thread.
    fn with_ui_thread<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        slint::invoke_from_event_loop(f).unwrap();
    }
}

impl IUiMethods for UiMethods {
    fn focus_device(&self, device_strategy: &dyn DeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        self.with_main_window(move |main_window| {
            device_strategy.focus_device(main_window);
        });
    }

    fn get_selected_device_index(&self, device_strategy: &dyn DeviceStrategy) -> usize {
        trace!("get_selected_device_index: {:?}", device_strategy.device_type());
        let device_strategy = device_strategy.clone_box();
        let index = self.with_main_window_result(move |main_window| {
            device_strategy.get_selected_device_index(main_window) as usize
        });
        trace!("get_selected_device_index: returning selected device index {}", index);
        index
    }

    fn set_selected_device_index(&self, index: usize, device_strategy: &dyn DeviceStrategy) {
        trace!("set_selected_device_index: index = {}, device_strategy = {:?}", index, device_strategy.device_type());
        let device_strategy = device_strategy.clone_box();
        self.with_main_window(move |main_window| {
            trace!("set_selected_device_index: Setting selected device index");
            device_strategy.set_selected_device_index(main_window, index as i32);
        });
    }

    fn set_devices_model(&self, device_names: &[String], device_strategy: &dyn DeviceStrategy) {
        trace!("set_devices_model: START");
        trace!("set_devices_model: Creating device items from port names");
        let device_items: Vec<ComboBoxItem> =
            device_names
                .iter()
                .map(|text| ComboBoxItem { text: text.into() })
                .collect();
        trace!("set_devices_model: Getting device type");
        let device_type = *device_strategy.device_type();
        trace!("set_devices_model: Cloning device_strategy");
        let device_strategy = device_strategy.clone_box();
        trace!("set_devices_model: Calling with_main_window");
        self.with_main_window(move |main_window| {
            trace!("set_devices_model: Inside with_main_window closure");
            let model = match device_type {
                DeviceType::Input => {
                    let input_model = Rc::new(MainComboBoxModel(device_items.clone()));
                    slint::ModelRc::from(input_model)
                },
                DeviceType::Output => {
                    let output_model = Rc::new(MainComboBoxModel(device_items.clone()));
                    slint::ModelRc::from(output_model)
                },
            };
            trace!("set_devices_model: Calling device_strategy.set_devices_model");
            device_strategy.set_devices_model(main_window, model);
            trace!("set_devices_model: Done with device_strategy.set_devices_model");
        });
        trace!("set_devices_model: END");
    }

    fn show_connected_device_name(&self, name: &str, message_type: MessageType,
                                device_strategy: &dyn DeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        let device_name = name.to_string();
        self.with_main_window(move |main_window| {
            device_strategy.show_connected_device_name(
                main_window, &device_name, slint_message_type(message_type));
        });
    }

    fn show_message(&self, message: &str, message_type: MessageType) {
        trace!("show_message: {}", message);
        let message = message.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_message(message.into(), slint_message_type(message_type));
        });
    }


    fn show_new_version_window(&self, new_version: &str, auto_check_new_versions: bool,
                               callbacks: Arc<Mutex<dyn NewVersionCallbacks>>) {
        let new_version_string = new_version.to_string();
        let new_version_window_weak = self.new_version_window_weak.clone();
        let self_clone = self.clone();
        self.with_ui_thread(move || {
            let dialog_borrow = new_version_window_weak.upgrade().unwrap();
            // Re-setup callbacks each time as the window might be reused
            dialog_borrow.set_auto_check_new_versions(auto_check_new_versions);
            dialog_borrow.set_message(
                format!("Version {} of {} is available.", new_version_string, APP_TITLE).into());

            let callbacks_clone_1 = callbacks.clone();
            dialog_borrow.on_auto_check_new_versions_changed(move |auto_check_new_versions: bool| {
                callbacks_clone_1.lock().unwrap().on_auto_check_new_versions_changed(
                    auto_check_new_versions);
            });
            let callbacks_clone_2 = callbacks.clone();
            dialog_borrow.on_ignore_this_version(move || {
                callbacks_clone_2.lock().unwrap().on_ignore_this_version(
                    new_version_string.clone());
            });
            dialog_borrow.on_close_window({
                let dialog_weak = dialog_borrow.as_weak();
                move || { dialog_weak.unwrap().hide().unwrap(); }
            });

            self_clone.show_dialog_in_centre_of_main_window(&dialog_borrow);
        });
    }

    fn show_pitchgrid_status(&self, status: &str, message_type: MessageType) {
        let message = status.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_pitchgrid_status(message.into(),
                                                     slint_message_type(message_type));
        });
    }

    fn show_tuning(&self, tuning: FormattedTuningParams, is_root_freq_overridden: bool) {
        trace!("show_tuning");
        self.with_main_window(move |main_window| {
            main_window.set_root_freq(tuning.root_freq.into());
            main_window.set_stretch(tuning.stretch.into());
            main_window.set_skew(tuning.skew.into());
            main_window.set_mode_offset(tuning.mode_offset.into());
            main_window.set_steps(tuning.steps.into());
            let mos = {
                if !tuning.mos_large_step_count.is_empty() {
                    format!("{}L {}s",
                    tuning.mos_large_step_count, tuning.mos_small_step_count)
                } else {
                    String::new()
                }
            };
            main_window.set_mos(mos.into());
            main_window.set_root_freq_overridden(is_root_freq_overridden);
        });
    }

    fn set_main_window_position(&self, x: i32, y: i32) {
        self.with_main_window(move |main_window| {
            main_window.window().set_position(slint::PhysicalPosition { x, y });
        });
    }

    fn set_override_rounding_initial(&self, value: bool) {
        self.with_main_window(move |main_window| {
            main_window.set_override_rounding_initial(value);
        });
    }

    fn set_override_rounding_rate(&self, value: bool) {
        self.with_main_window(move |main_window| {
            main_window.set_override_rounding_rate(value);
        });
    }

    fn set_root_freq_override_index(&self, index: usize) {
        self.with_main_window(move |main_window| {
            main_window.set_root_freq_override_index(index as i32);
        });
    }

    fn set_rounding_rate(&self, rate: u8) {
        self.with_main_window(move |main_window| {
            main_window.set_rounding_rate(rate as i32);
        });
    }

    fn set_selected_osc_listening_port_index(&self, index: usize) {
        self.with_main_window(move |main_window| {
            main_window.set_selected_osc_listening_port_index(index as i32);
        });
    }

    fn set_selected_pitch_table_index(&self, index: usize) {
        self.with_main_window(move |main_window| {
            main_window.set_selected_pitch_table_index(index as i32);
        });
    }
}



fn slint_message_type(message_type: MessageType) -> SlintMessageType {
    match message_type {
        MessageType::Info => SlintMessageType::Info,
        MessageType::Warning => SlintMessageType::Warning,
        MessageType::Error => SlintMessageType::Error,
    }
}
