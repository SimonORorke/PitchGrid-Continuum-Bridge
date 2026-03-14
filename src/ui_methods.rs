use std::rc::Rc;
use slint::Weak;
use crate::{MainWindow, ComboBoxItem, SlintMessageType, InputPortsModel as MainInputPortsModel,
            OutputPortsModel as MainOutputPortsModel};
use crate::controller::ControllerCallbacks;
use crate::global::{MessageType, PortType};
use crate::port_strategy::PortStrategy;
use crate::tuner;

/// This struct contains the methods called by Controller to make changes to the UI.
/// It is part of the view in the Model-View-Controller (MVC) pattern.
/// See Controller's doc comment for more information on how the project implements MVC.
pub struct UiMethods {
    main_window: Weak<MainWindow>,
}

impl UiMethods {
    pub fn new(main_window: Weak<MainWindow>) -> Self {
        Self { main_window }
    }

    fn with_main_window<F>(&self, f: F)
    where
        F: FnOnce(&MainWindow),
    {
        // println!("UiMethods.with_main_window: Attempting to upgrade main_window");
        if let Some(main_window) = self.main_window.upgrade() {
            // println!("UiMethods.with_main_window: Successfully upgraded, calling closure");
            f(&main_window);
            // println!("UiMethods.with_main_window: Closure completed");
        } else {
            // println!("UiMethods.with_main_window: Failed to upgrade main_window");
        }
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
        if let Some(main_window) = self.main_window.upgrade() {
            port_strategy.get_selected_port_index(&main_window) as usize
        } else {
            0
        }
    }

    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy) {
        let port_strategy = port_strategy.clone_box();
        self.with_main_window(move |main_window| {
            port_strategy.set_selected_port_index(main_window, index as i32);
        });
    }

    fn set_ports_model(&self, port_names: &Vec<String>, port_strategy: &dyn PortStrategy) {
        // println!("UiMethods.set_ports_model: START");
        // println!("UiMethods.set_ports_model: Creating port items from port names");
        let port_items: Vec<ComboBoxItem> =
            port_names
                .iter()
                .map(|text| ComboBoxItem { text: text.into() })
                .collect();
        // println!("UiMethods.set_ports_model: Getting port type");
        let port_type = port_strategy.port_type().clone();
        // println!("UiMethods.set_ports_model: Cloning port_strategy");
        let port_strategy = port_strategy.clone_box();
        // println!("UiMethods.set_ports_model: Calling with_main_window");
        self.with_main_window(move |main_window| {
            // println!("UiMethods.set_ports_model: Inside with_main_window closure");
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
            // println!("UiMethods.set_ports_model: Calling port_strategy.set_ports_model");
            port_strategy.set_ports_model(main_window, model);
            // println!("UiMethods.set_ports_model: Done with port_strategy.set_ports_model");
        });
        // println!("UiMethods.set_ports_model: END");
    }

    fn show_connected_port_name(&self, name: &str, message_type: MessageType, port_strategy: &dyn PortStrategy) {
        let port_strategy = port_strategy.clone_box();
        let port_name = name.to_string();
        self.with_main_window(move |main_window| {
            port_strategy.show_connected_port_name(
                main_window, &port_name, slint_message_type(message_type));
        });
    }

    fn show_message(&self, message: &str, message_type: MessageType) {
        // println!("UiMethods.show_message: {}", message);
        let message = message.to_string();
        let weak = self.main_window.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(main_window) = weak.upgrade() {
                // println!("UiMethods.show_message: Got main_window. Calling main_window.invoke_show_message");
                main_window.invoke_show_message(message.into(), slint_message_type(message_type));
                // println!("UiMethods.show_message: Message shown");
            } else {
                // println!("UiMethods.show_message: Failed to upgrade main_window");
            }
        }).unwrap();
    }

    fn show_pitchgrid_status(&self, status: &str, message_type: MessageType) {
        let message = status.to_string();
        let weak = self.main_window.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(main_window) = weak.upgrade() {
                main_window.invoke_show_pitchgrid_status(message.into(),
                                                         slint_message_type(message_type));
            }
        }).unwrap();
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
