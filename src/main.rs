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
mod ui_methods;
mod midi_static;

use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use slint::{CloseRequestResponse, Weak};
use controller::{Controller};
use global::{APP_TITLE, VERSION};
use port_strategy::{
    InputStrategy, OutputStrategy, PortStrategy};
use ui_methods::UiMethods;

/// main.rs is part of the view in the Model-View-Controller (MVC) pattern.
/// See Controller's doc comment for more information on how the project implements MVC.
fn main() {
    let main_window = MainWindow::new().unwrap();
    *MAIN_WINDOW_WEAK.lock().unwrap() = Some(main_window.as_weak().clone());
    main_window.set_window_title(format!("{} v{}", APP_TITLE, VERSION).into());
    // main_window.set_window_title(APP_TITLE.into());
    let ui_methods = UiMethods::new(main_window.as_weak());
    let controller: SharedController = Arc::new(Mutex::new(Controller::new(
        Box::new(ui_methods)
    )));
    Controller::set_controller(controller.clone());
    init_ui_handlers(&main_window, controller.clone());
    set_roundings_model(&main_window);
    set_overrides_model(&main_window);
    set_pitch_tables_model(&main_window);

    // Initialize controller on a background thread so that UI callbacks within
    // init() can use invoke_from_event_loop without deadlocking.
    let controller_clone = controller.clone();
    rayon::spawn(move || {
        controller_clone.lock().unwrap().init();
    });

    main_window.run().unwrap();
}

fn init_ui_handlers(main_window: &MainWindow, controller: SharedController) {
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.window().on_close_requested(move || {
            handle_close_request(&controller)
        });
    }
    // All Controller methods must be called from non-UI threads to avoid deadlock.
    // See the UiMethods.with_main_window_result doc comment for more information.
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_connect_port(move |port_type: SlintPortType| {
            let controller = controller.clone();
            let port_strategy = create_port_strategy(port_type);
            rayon::spawn(move || {
                controller.lock().unwrap().connect_port(&*port_strategy);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_refresh_devices(move |port_type: SlintPortType| {
            let controller = controller.clone();
            let port_strategy = create_port_strategy(port_type);
            rayon::spawn(move || {
                controller.lock().unwrap().refresh_devices(&*port_strategy);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_override_changed(move |index| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_root_freq_override(index as usize);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_rounding_changed(move |rounding_index| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_rounding(rounding_index as usize);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_pitch_table_changed(move |index| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_pitch_table_no(index as usize);
            });
        });
    }
}

fn handle_close_request(controller: &SharedController) -> CloseRequestResponse {
    // println!("main.handle_close_request");
    let response =
        Arc::new(Mutex::new(CloseRequestResponse::HideWindow));
    if IS_CLOSE_ERROR_SHOWN.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return *response.lock().unwrap()
    }
    let response_clone = Arc::clone(&response);
    // Controller.close() won't deadlock, as it does not access the GUI.
    // So it is safe to call it from the UI event loop thread.
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

fn set_overrides_model(main_window: &MainWindow) {
    let override_items: Vec<ComboBoxItem> = global::override_note_names()
        .iter()
        .map(|override_name| ComboBoxItem { text: override_name.into() })
        .collect();
    let model = Rc::new(ComboBoxModel(override_items));
    main_window.set_overrides_model(slint::ModelRc::from(model));
}

fn set_roundings_model(main_window: &MainWindow) {
    let rounding_items: Vec<ComboBoxItem> = global::rounding_names()
        .iter()
        .map(|rounding_name| ComboBoxItem { text: rounding_name.into() })
        .collect();
    let model = Rc::new(ComboBoxModel(rounding_items));
    main_window.set_roundings_model(slint::ModelRc::from(model));
}

fn set_pitch_tables_model(main_window: &MainWindow) {
    let pitch_table_items: Vec<ComboBoxItem> = tuner::pitch_table_nos()
        .iter()
        .map(|grid_no| ComboBoxItem { text: grid_no.to_string().into() })
        .collect();
    let model = Rc::new(ComboBoxModel(pitch_table_items));
    main_window.set_pitch_tables_model(slint::ModelRc::from(model));
}

type SharedController = Arc<Mutex<Controller>>;

pub struct ComboBoxModel(pub Vec<ComboBoxItem>);

impl slint::Model for ComboBoxModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0.get(row).cloned()
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

slint::include_modules!();

lazy_static! {
    static ref IS_CLOSE_ERROR_SHOWN: AtomicBool = AtomicBool::new(false);
    static ref MAIN_WINDOW_WEAK: Mutex<Option<Weak<MainWindow>>> = Mutex::new(None);
}
