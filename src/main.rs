// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod global;
mod midi;
mod midi_ports;
mod osc;
mod settings;
mod tuner;
mod port_strategy;
mod controller;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use slint::{CloseRequestResponse, Weak};
use controller::{Controller, set_controller};
use global::{APP_TITLE, MessageType, PortType};
use port_strategy::{
    InputStrategy, OutputStrategy, PortStrategy};

fn main() {
    let main_window = MainWindow::new().unwrap();
    *MAIN_WINDOW_WEAK.lock().unwrap() = Some(main_window.as_weak().clone());
    main_window.set_window_title(APP_TITLE.into());
    let controller: SharedController = Arc::new(Mutex::new(Controller::new(
        Box::from(focus_port),
        Box::from(get_selected_port_index),
        Box::from(set_selected_port_index),
        Box::from(set_ports_model),
        Box::from(show_connected_port_name),
        Box::from(show_message),
        Box::from(show_pitchgrid_status),
        Box::from(show_tuning),
        Box::from(set_selected_pitch_table_index)
    )));
    set_controller(controller.clone());
    init_ui_handlers(&main_window, controller.clone());
    set_pitch_tables_model(&main_window);
    controller.lock().unwrap().init();
    main_window.run().unwrap();
}

fn init_ui_handlers(main_window: &MainWindow, controller: SharedController) {
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.window().on_close_requested(move || {
            handle_close_request(&controller)
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_connect_port(move |port_type: SlintPortType| {
            let port_strategy = create_port_strategy(port_type);
            controller.lock().unwrap().connect_port(&*port_strategy)
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_refresh_ports(move |port_type: SlintPortType| {
            let port_strategy = create_port_strategy(port_type);
            controller.lock().unwrap().refresh_ports(&*port_strategy)
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_pitch_table_changed(move |index| {
            controller.lock().unwrap().set_pitch_table_no(index as usize)
        });
    }
}

fn handle_close_request(controller: &SharedController) -> CloseRequestResponse {
    println!("main.handle_close_request");
    let response =
        Arc::new(Mutex::new(CloseRequestResponse::HideWindow));
    if IS_CLOSE_ERROR_SHOWN.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return *response.lock().unwrap()
    }
    let response_clone = Arc::clone(&response);
    if let Err(_) = controller.lock().unwrap().close() {
        *response_clone.lock().unwrap() = CloseRequestResponse::KeepWindowShown;
        IS_CLOSE_ERROR_SHOWN.store(true, Ordering::Relaxed);
    };
    *response.lock().unwrap()
}

fn create_port_strategy(port_type: SlintPortType)
                        -> Box<dyn PortStrategy> {
    match port_type {
        SlintPortType::Input => InputStrategy::new().clone_box(),
        SlintPortType::Output => OutputStrategy::new().clone_box(),
    }
}

fn main_window_weak_static_clone() -> Option<Weak<MainWindow>> {
    MAIN_WINDOW_WEAK.lock().unwrap().clone()
}

fn focus_port(port_strategy: &dyn PortStrategy) {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        let port_strategy = port_strategy.clone_box();
        with_main_window(main_window_weak.clone(), move |main_window| {
            port_strategy.focus_port(main_window);
        });
    }
}

fn get_selected_port_index(port_strategy: &dyn PortStrategy) -> usize {
    let result = Arc::new(Mutex::new(0usize));
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        let port_strategy = port_strategy.clone_box();
        let result_clone = result.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(main_window) = main_window_weak.upgrade() {
                *result_clone.lock().unwrap() = port_strategy.get_selected_port_index(&main_window) as usize;
            }
        });
    }
    *result.lock().unwrap()
}

fn set_selected_port_index(index: usize, port_strategy: &dyn PortStrategy) {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        let port_strategy = port_strategy.clone_box();
        with_main_window(main_window_weak.clone(), move |main_window| {
            port_strategy.set_selected_port_index(main_window, index as i32);
        });
    }
}

fn set_selected_pitch_table_index(index: i32) {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        with_main_window(main_window_weak.clone(), move |main_window| {
            main_window.set_selected_pitch_table_index(index);
        });
    }
}

fn set_ports_model(controller: &Controller, port_strategy: &dyn PortStrategy) {
    let port_items: Vec<ComboBoxItem> =
        controller.port_names(port_strategy)
            .iter()
            .map(|text| ComboBoxItem { text: text.into() })
            .collect();
    let port_type = port_strategy.port_type().clone();
    let port_strategy = port_strategy.clone_box();
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        with_main_window(main_window_weak.clone(), move |main_window| {
            let model = match port_type {
                PortType::Input => {
                    let input_model = Rc::new(InputPortsModel(port_items.clone()));
                    slint::ModelRc::from(input_model)
                },
                PortType::Output => {
                    let output_model = Rc::new(OutputPortsModel(port_items.clone()));
                    slint::ModelRc::from(output_model)
                },
            };
            port_strategy.set_ports_model(main_window, model);
        });
    }
}

fn show_connected_port_name(
    port_name: &str, message_type: MessageType, port_strategy: &dyn PortStrategy) {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        let port_strategy = port_strategy.clone_box();
        let port_name = port_name.to_string();
        with_main_window(main_window_weak.clone(), move |main_window| {
            port_strategy.show_connected_port_name(
                main_window, &port_name, slint_message_type(message_type));
        });
    }
}

fn show_message(message: &str, message_type: MessageType) {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        let message = message.to_string();
        with_main_window(main_window_weak.clone(), move |main_window| {
            main_window.invoke_show_message(message.into(), slint_message_type(message_type));
        });
    }
}

fn show_pitchgrid_status(message: &str, message_type: MessageType) {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        let message = message.to_string();
        with_main_window(main_window_weak.clone(), move |main_window| {
            main_window.invoke_show_pitchgrid_status(message.into(),
                                                     slint_message_type(message_type));
        });
    }
}

fn show_tuning() {
    if let Some(main_window_weak) = main_window_weak_static_clone() {
        with_main_window(main_window_weak.clone(), move |main_window| {
            let params = tuner::formatted_tuning_params();
            main_window.set_depth(params.depth.into());
            main_window.set_root_freq(params.root_freq.into());
            main_window.set_stretch(params.stretch.into());
            main_window.set_skew(params.skew.into());
            main_window.set_mode_offset(params.mode_offset.into());
            main_window.set_steps(params.steps.into());
        });
    }
}

fn set_pitch_tables_model(main_window: &MainWindow) {
    let pitch_table_items: Vec<ComboBoxItem> = tuner::pitch_table_nos()
        .iter()
        .map(|grid_no| ComboBoxItem { text: grid_no.to_string().into() })
        .collect();
    let model = Rc::new(TuningGridsModel(pitch_table_items));
    main_window.set_pitch_tables_model(slint::ModelRc::from(model));
}

fn slint_message_type(message_type: MessageType) -> SlintMessageType {
   match message_type {
       MessageType::Info => SlintMessageType::Info,
       MessageType::Warning => SlintMessageType::Warning,
       MessageType::Error => SlintMessageType::Error,
   }
}

/// Upgrades a weak reference to a `MainWindow` to a strong reference, then calls the closure with
/// the strong reference. If called from a non-UI thread, the closure will be executed in the UI
/// event loop.
fn with_main_window(main_window_weak: Weak<MainWindow>,
                    f: impl FnOnce(&MainWindow) + Send + 'static) {
    main_window_weak.upgrade_in_event_loop(move |main_window| {
        f(&main_window);
    }).ok();
}

type SharedController = Arc<Mutex<Controller>>;

struct InputPortsModel(Vec<ComboBoxItem>);

impl slint::Model for InputPortsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) ->
        Option<Self::Data> {
        self.0.get(row).map(|x| x.clone())
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

struct OutputPortsModel(Vec<ComboBoxItem>);

impl slint::Model for OutputPortsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) ->
        Option<Self::Data> {
        self.0.get(row).map(|x| x.clone())
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

struct TuningGridsModel(Vec<ComboBoxItem>);

impl slint::Model for TuningGridsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) ->
        Option<Self::Data> {
        self.0.get(row).map(|x| x.clone())
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

// const PORT_NONE: &str = "[None]";

slint::include_modules!();

lazy_static! {
    static ref IS_CLOSE_ERROR_SHOWN: AtomicBool = AtomicBool::new(false);
    static ref MAIN_WINDOW_WEAK: Mutex<Option<Weak<MainWindow>>> = Mutex::new(None);
}
