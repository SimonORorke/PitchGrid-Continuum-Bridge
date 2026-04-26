use std::rc::Rc;
use slint::{ComponentHandle, Weak};
use crate::{MainWindow, ComboBoxItem, SlintMessageType, ComboBoxModel as MainComboBoxModel};
use crate::controller::ControllerCallbacks;
use crate::global::{MessageType, PortType};
use crate::port_strategy::PortStrategy;
use crate::tuner;

/// This struct contains the methods called by Controller to make changes to the UI.
/// It is part of the view in the Model-View-Controller (MVC) pattern.
/// See Controller's doc comment for more information on how the project implements MVC.
pub struct UiMethods {
    main_window_weak: Weak<MainWindow>,
}

impl UiMethods {
    pub fn new(main_window_weak: Weak<MainWindow>) -> Self {
        Self { main_window_weak }
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
}

impl ControllerCallbacks for UiMethods {
    fn focus_port(&self, port_strategy: &dyn PortStrategy) {
        let port_strategy = port_strategy.clone_box();
        self.with_main_window(move |main_window| {
            port_strategy.focus_port(main_window);
        });
    }

    fn get_selected_port_index(&self, port_strategy: &dyn PortStrategy) -> usize {
        // println!("UiMethods.get_selected_port_index: {:?}", port_strategy.port_type());
        let port_strategy = port_strategy.clone_box();
        let index = self.with_main_window_result(move |main_window| {
            port_strategy.get_selected_port_index(main_window) as usize
        });
        // println!("UiMethods.get_selected_port_index: returning selected port index {}", index);
        index
    }

    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy) {
        // println!("UiMethods.set_selected_port_index: index = {}, port_strategy = {:?}", index, port_strategy.port_type());
        let port_strategy = port_strategy.clone_box();
        self.with_main_window(move |main_window| {
            // println!("UiMethods.set_selected_port_index: Setting selected port index");
            port_strategy.set_selected_port_index(main_window, index as i32);
        });
    }

    fn set_devices_model(&self, device_names: &Vec<String>, port_strategy: &dyn PortStrategy) {
        // println!("UiMethods.set_devices_model: START");
        // println!("UiMethods.set_devices_model: Creating port items from port names");
        let device_items: Vec<ComboBoxItem> =
            device_names
                .iter()
                .map(|text| ComboBoxItem { text: text.into() })
                .collect();
        // println!("UiMethods.set_devices_model: Getting port type");
        let port_type = port_strategy.port_type().clone();
        // println!("UiMethods.set_devices_model: Cloning port_strategy");
        let port_strategy = port_strategy.clone_box();
        // println!("UiMethods.set_devices_model: Calling with_main_window");
        self.with_main_window(move |main_window| {
            // println!("UiMethods.set_devices_model: Inside with_main_window closure");
            let model = match port_type {
                PortType::Input => {
                    let input_model = Rc::new(MainComboBoxModel(device_items.clone()));
                    slint::ModelRc::from(input_model)
                },
                PortType::Output => {
                    let output_model = Rc::new(MainComboBoxModel(device_items.clone()));
                    slint::ModelRc::from(output_model)
                },
            };
            // println!("UiMethods.set_devices_model: Calling port_strategy.set_devices_model");
            port_strategy.set_devices_model(main_window, model);
            // println!("UiMethods.set_devices_model: Done with port_strategy.set_devices_model");
        });
        // println!("UiMethods.set_devices_model: END");
    }

    fn show_connected_device_name(&self, name: &str, message_type: MessageType,
                                port_strategy: &dyn PortStrategy) {
        let port_strategy = port_strategy.clone_box();
        let device_name = name.to_string();
        self.with_main_window(move |main_window| {
            port_strategy.show_connected_device_name(
                main_window, &device_name, slint_message_type(message_type));
        });
    }

    fn show_message(&self, message: &str, message_type: MessageType) {
        // println!("UiMethods.show_message: {}", message);
        let message = message.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_message(message.into(), slint_message_type(message_type));
        });
    }

    fn show_pitchgrid_status(&self, status: &str, message_type: MessageType) {
        let message = status.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_pitchgrid_status(message.into(),
                                                     slint_message_type(message_type));
        });
    }

    fn show_tuning(&self, is_root_freq_overridden: bool) {
        // println!("UiMethods.show_tuning");
        self.with_main_window(move |main_window| {
            let params = tuner::formatted_tuning_params();
            main_window.set_root_freq(params.root_freq.into());
            main_window.set_stretch(params.stretch.into());
            main_window.set_skew(params.skew.into());
            main_window.set_mode_offset(params.mode_offset.into());
            main_window.set_steps(params.steps.into());
            let mos = format!("{}L {}s",
                              params.mos_large_step_count, params.mos_small_step_count);
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

    fn set_rounding_rate(&self, rate: u8) {
        self.with_main_window(move |main_window| {
            main_window.set_rounding_rate(rate as i32);
        });
    }

    fn set_selected_osc_listening_port_index(&self, index: i32) {
        self.with_main_window(move |main_window| {
            main_window.set_selected_osc_listening_port_index(index);
        });
    }

    fn set_selected_pitch_table_index(&self, index: i32) {
        self.with_main_window(move |main_window| {
            main_window.set_selected_pitch_table_index(index);
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
