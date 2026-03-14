use std::rc::Rc;
use std::sync::{Arc, Mutex};
use slint::Weak;
use crate::{MainWindow, ComboBoxItem, SlintMessageType, InputPortsModel as MainInputPortsModel,
            OutputPortsModel as MainOutputPortsModel};
use crate::controller::{Controller, ControllerCallbacks};
use crate::global::{MessageType, PortType};
use crate::port_strategy::PortStrategy;
use crate::tuner;

pub struct UiMethods {
    main_window: Weak<MainWindow>,
}

impl UiMethods {
    pub fn new(main_window: Weak<MainWindow>) -> Self {
        Self { main_window }
    }

    fn with_main_window<F>(&self, f: F)
    where
        F: FnOnce(&MainWindow) + Send + 'static,
    {
        let main_window_weak = self.main_window.clone();
        main_window_weak.upgrade_in_event_loop(move |main_window| {
            f(&main_window);
        }).ok();
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
        let result = Arc::new(Mutex::new(0usize));
        let port_strategy = port_strategy.clone_box();
        let result_clone = result.clone();
        let main_window_weak = self.main_window.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(main_window) = main_window_weak.upgrade() {
                *result_clone.lock().unwrap() = port_strategy.get_selected_port_index(&main_window) as usize;
            }
        });
        *result.lock().unwrap()
    }

    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy) {
        let port_strategy = port_strategy.clone_box();
        self.with_main_window(move |main_window| {
            port_strategy.set_selected_port_index(main_window, index as i32);
        });
    }

    fn set_ports_model(&self, controller: &Controller, port_strategy: &dyn PortStrategy) {
        let port_items: Vec<ComboBoxItem> =
            controller.port_names(port_strategy)
                .iter()
                .map(|text| ComboBoxItem { text: text.into() })
                .collect();
        let port_type = port_strategy.port_type().clone();
        let port_strategy = port_strategy.clone_box();
        self.with_main_window(move |main_window| {
            let model = match port_type {
                PortType::Input => {
                    let input_model = Rc::new(MainInputPortsModel(port_items.clone()));
                    slint::ModelRc::from(input_model)
                },
                PortType::Output => {
                    let output_model = Rc::new(MainOutputPortsModel(port_items.clone()));
                    slint::ModelRc::from(output_model)
                },
            };
            port_strategy.set_ports_model(main_window, model);
        });
    }

    fn show_connected_port_name(&self, name: &str, msg_type: MessageType, port_strategy: &dyn PortStrategy) {
        let port_strategy = port_strategy.clone_box();
        let port_name = name.to_string();
        self.with_main_window(move |main_window| {
            port_strategy.show_connected_port_name(
                main_window, &port_name, slint_message_type(msg_type));
        });
    }

    fn show_message(&self, msg: &str, msg_type: MessageType) {
        let message = msg.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_message(message.into(), slint_message_type(msg_type));
        });
    }

    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType) {
        let message = status.to_string();
        self.with_main_window(move |main_window| {
            main_window.invoke_show_pitchgrid_status(message.into(),
                                                     slint_message_type(msg_type));
        });
    }

    fn show_tuning(&self) {
        self.with_main_window(move |main_window| {
            let params = tuner::formatted_tuning_params();
            main_window.set_depth(params.depth.into());
            main_window.set_root_freq(params.root_freq.into());
            main_window.set_stretch(params.stretch.into());
            main_window.set_skew(params.skew.into());
            main_window.set_mode_offset(params.mode_offset.into());
            main_window.set_steps(params.steps.into());
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
