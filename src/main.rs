// Prevent console window in addition to Slint window in Windows release builds when, e.g. starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use slint::{CloseRequestResponse, ComponentHandle, PhysicalPosition, WindowPosition, Weak};
use app_info::{APP_TITLE, COPYRIGHT, DOCUMENTATION_LINK, LICENSE, PROJECT_LINK, VERSION};
use pitchgrid_continuum::{ComboBoxModel, ComboBoxItem, MainWindow, AboutWindow, SharedController, SlintDeviceType};
use pitchgrid_continuum::controller::Controller;
use pitchgrid_continuum::osc::Osc;
use pitchgrid_continuum::device_strategy::{InputStrategy, OutputStrategy, DeviceStrategy};
use pitchgrid_continuum::ui_methods::UiMethods;
use pitchgrid_continuum::global;
use pitchgrid_continuum::tuner::Tuner;
use log::trace;

/// main.rs is part of the view in the Model-View-Controller (MVC) pattern.
/// See `Controller`'s doc comment for more information on how the project implements MVC.
fn main() {
    // Initialise logging. Levels are chosen at runtime via the RUST_LOG env var
    // (e.g. `RUST_LOG=debug`, or `RUST_LOG=pitchgrid_continuum::tuner=trace`); with RUST_LOG unset
    // the default filter is `info`, so the breadcrumb `debug!`/`trace!` lines stay silent.
    // `log` is a facade: this init line is the only place that names `env_logger`, so switching to a
    // file logger later (handoff §6) changes just this line, not the call sites.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"))
            .format_timestamp_millis().init();
    #[cfg(target_os = "macos")]
    set_macos_app_icon();
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title(APP_TITLE.into());
    let ui_methods = UiMethods::new(main_window.as_weak());
    let controller: SharedController = Arc::new(Mutex::new(Controller::new(
        Arc::new(ui_methods)
    )));
    init_ui_handlers(&main_window, controller.clone());
    set_root_notes_model(&main_window);
    set_osc_listening_ports_model(&main_window);
    set_pitch_tables_model(&main_window);

    // Initialise controller on a background thread so that UI callbacks within
    // init() can use invoke_from_event_loop without deadlocking.
    let controller_clone = controller.clone();
    rayon::spawn(move || {
        let self_arc = controller_clone.clone();
        controller_clone.lock().unwrap().init(&self_arc);
    });

    main_window.run().unwrap();
}

fn init_ui_handlers(main_window: &MainWindow, controller: SharedController) {
    main_window.on_open_documentation(move || {
        open::that(DOCUMENTATION_LINK).unwrap();
    });
    let about_window: Rc<RefCell<Option<AboutWindow>>> = Rc::new(RefCell::new(None));
    {
        let about_window = Rc::clone(&about_window);
        let main_window_weak = main_window.as_weak();
        main_window.on_show_about_window(move || {
            let dialog = AboutWindow::new().unwrap();
            dialog.set_window_title(format!("About {}", APP_TITLE).into());
            dialog.set_app_title(APP_TITLE.into());
            dialog.set_version(VERSION.into());
            dialog.set_copyright(COPYRIGHT.into());
            dialog.set_license(LICENSE.into());
            dialog.set_project_link(PROJECT_LINK.into());
            dialog.on_open_project_link(|| {
                open::that(PROJECT_LINK).unwrap();
            });
            // Why is there a dialog.on_close_window before the dialog.show().unwrap()?
            // The callback must be registered before show() to avoid a race condition:
            // if the user somehow closed the window during or immediately after show() returned,
            // the close handler needs to already be in place. More practically, it's just the
            // conventional setup pattern — configure all properties and callbacks first,
            // then show. In practice for a dialog like this it makes no functional difference,
            // but registering handlers before showing is the safe, idiomatic order.
            dialog.on_close_window({
                let dialog_weak = dialog.as_weak();
                move || { dialog_weak.unwrap().hide().unwrap(); }
            });
            // Position the dialog in the centre of the main window.
            // We have to show the dialog first before we can position it.
            dialog.show().unwrap();
            if let Some(main_window) = main_window_weak.upgrade() {
                let mp = main_window.window().position();
                let ms = main_window.window().size();
                let scale = main_window.window().scale_factor();
                let dw = (dialog.get_preferred_w() * scale) as i32;
                let dh = (dialog.get_preferred_h() * scale) as i32;
                let x = mp.x + (ms.width as i32 - dw) / 2;
                let y = mp.y + (ms.height as i32 - dh) / 2;
                dialog.window().set_position(WindowPosition::Physical(PhysicalPosition { x, y }));
            }
            *about_window.borrow_mut() = Some(dialog);
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        let main_window_weak = main_window.as_weak();
        let about_window = Rc::clone(&about_window);
        main_window.window().on_close_requested(move || {
            handle_close_request(&main_window_weak, &controller, &about_window)
        });
    }
    // All Controller methods must be called from non-UI threads to avoid deadlock.
    // See the UiMethods.with_main_window_result doc comment for more information.
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_connect_device(move |device_type: SlintDeviceType| {
            let controller = controller.clone();
            let device_strategy = create_device_strategy(device_type);
            rayon::spawn(move || {
                controller.lock().unwrap().connect_device(&*device_strategy);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_refresh_devices(move |device_type: SlintDeviceType| {
            let controller = controller.clone();
            let device_strategy = create_device_strategy(device_type);
            rayon::spawn(move || {
                controller.lock().unwrap().refresh_devices(&*device_strategy);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_root_note_changed(move |index| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_root_freq_override(index as usize);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_override_rounding_initial_changed(move |override_rounding_initial| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_override_rounding_initial(override_rounding_initial);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_override_rounding_rate_changed(move |override_rounding_rate| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_override_rounding_rate(override_rounding_rate);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_rounding_rate_changed(move |rounding_rate| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_rounding_rate(rounding_rate as u8);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_osc_listening_port_changed(move |index| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_osc_listening_port(index as usize);
            });
        });
    }
    {
        let controller: SharedController = Arc::clone(&controller);
        main_window.on_selected_pitch_table_changed(move |index| {
            let controller = controller.clone();
            rayon::spawn(move || {
                controller.lock().unwrap().set_pitch_table(index as usize);
            });
        });
    }
}

fn handle_close_request(main_window_weak: &Weak<MainWindow>, controller: &SharedController, about_window: &Rc<RefCell<Option<AboutWindow>>>) -> CloseRequestResponse {
    trace!("main.handle_close_request");
    if let Some(dialog) = about_window.borrow().as_ref()
        && dialog.window().is_visible()
    {
        dialog.hide().unwrap();
    }
    let response =
        Arc::new(Mutex::new(CloseRequestResponse::HideWindow));
    if IS_CLOSE_ERROR_SHOWN.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return *response.lock().unwrap()
    }
    // Read position on the UI thread before calling close(), which runs on the UI thread
    // and cannot use invoke_from_event_loop without deadlocking.
    let (x, y) = if let Some(main_window) = main_window_weak.upgrade() {
        let pos = main_window.window().position();
        (pos.x, pos.y)
    } else {
        (0, 0)
    };
    let response_clone = Arc::clone(&response);
    if controller.lock().unwrap().close(x, y).is_err() {
        *response_clone.lock().unwrap() = CloseRequestResponse::KeepWindowShown;
        IS_CLOSE_ERROR_SHOWN.store(true, Ordering::Relaxed);
    };
    *response.lock().unwrap()
}

fn create_device_strategy(device_type: SlintDeviceType)
                        -> Box<dyn DeviceStrategy> {
    match device_type {
        SlintDeviceType::Input => InputStrategy::new().clone_box(),
        SlintDeviceType::Output => OutputStrategy::new().clone_box(),
    }
}

fn set_root_notes_model(main_window: &MainWindow) {
    let override_items: Vec<ComboBoxItem> = global::override_note_names()
        .iter()
        .map(|override_name| ComboBoxItem { text: override_name.into() })
        .collect();
    let model = Rc::new(ComboBoxModel(override_items));
    main_window.set_root_notes_model(slint::ModelRc::from(model));
}

fn set_osc_listening_ports_model(main_window: &MainWindow) {
    let osc_listening_port_items: Vec<ComboBoxItem> = Osc::listening_ports()
        .iter()
        .map(|port| ComboBoxItem { text: port.to_string().into() })
        .collect();
    let model = Rc::new(ComboBoxModel(osc_listening_port_items));
    main_window.set_osc_listening_ports_model(slint::ModelRc::from(model));
}

fn set_pitch_tables_model(main_window: &MainWindow) {
    let pitch_table_items: Vec<ComboBoxItem> = Tuner::pitch_tables()
        .iter()
        .map(|grid_no| ComboBoxItem { text: grid_no.to_string().into() })
        .collect();
    let model = Rc::new(ComboBoxModel(pitch_table_items));
    main_window.set_pitch_tables_model(slint::ModelRc::from(model));
}

#[cfg(target_os = "macos")]
fn set_macos_app_icon() {
    let icon_data = include_bytes!("../Midi port black on red 512.icns");
    unsafe {
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};
        let data: *mut Object = msg_send![
            class!(NSData),
            dataWithBytes: icon_data.as_ptr() as *const std::ffi::c_void
            length: icon_data.len()
        ];
        let image: *mut Object = msg_send![class!(NSImage), alloc];
        let image: *mut Object = msg_send![image, initWithData: data];
        let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, setApplicationIconImage: image];
    }
}

static IS_CLOSE_ERROR_SHOWN: AtomicBool = AtomicBool::new(false);
