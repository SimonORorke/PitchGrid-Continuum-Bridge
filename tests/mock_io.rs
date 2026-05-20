use std::cell::RefCell;
use std::error::Error;
use pitchgrid_continuum::midi_ports::{IIo, IoDevice};

/// Returns a clone of the current `IoState`.
pub fn io_state() -> IoState {
    IO_STATE.with(|s| s.borrow().clone())
}

pub struct MockIo {
    /// Controls the return value of `device()`. Set directly on the mock to configure.
    pub device: Option<MockDevice>,
}

impl MockIo {
    pub fn new() -> Self {
        IO_STATE.replace(IoState::new());
        MockIo { device: None }
    }
}

impl IIo for MockIo {
    #[allow(dead_code)]
    fn device(&self) -> Option<&dyn IoDevice> {
        IO_STATE.with_borrow_mut(|s| {
            s.device_count += 1;
        });
        self.device.as_ref().map(|d| d as &dyn IoDevice)
    }

    #[allow(dead_code)]
    fn device_names(&self) -> Vec<String> {
        IO_STATE.with_borrow_mut(|s| {
            s.device_names_count += 1;
        });
        IO_STATE.with(|s| s.borrow().device_names_result.clone())
    }

    #[allow(dead_code)]
    fn populate_devices(&mut self, persisted_device_name: &str) -> Result<(), Box<dyn Error>> {
        IO_STATE.with_borrow_mut(|s| {
            s.populate_devices_count += 1;
            s.populate_devices_persisted_device_name = Some(persisted_device_name.to_string());
        });
        if IO_STATE.with(|s| s.borrow().populate_devices_ok) {
            Ok(())
        } else {
            Err("mock error".into())
        }
    }
}

pub struct MockDevice {
    pub index: usize,
    pub name: String,
}

impl IoDevice for MockDevice {
    fn index(&self) -> usize { self.index }
    fn name(&self) -> String { self.name.clone() }
}

pub struct IoState {
    pub device_count: u16,

    pub device_names_count: u16,
    pub device_names_result: Vec<String>,

    pub populate_devices_count: u16,
    pub populate_devices_persisted_device_name: Option<String>,
    pub populate_devices_ok: bool,
}

impl IoState {
    pub fn new() -> Self {
        IoState {
            device_count: 0,

            device_names_count: 0,
            device_names_result: Vec::new(),

            populate_devices_count: 0,
            populate_devices_persisted_device_name: None,
            populate_devices_ok: true,
        }
    }
}

impl Clone for IoState {
    fn clone(&self) -> Self {
        IoState {
            device_count: self.device_count,

            device_names_count: self.device_names_count,
            device_names_result: self.device_names_result.clone(),

            populate_devices_count: self.populate_devices_count,
            populate_devices_persisted_device_name: self.populate_devices_persisted_device_name.clone(),
            populate_devices_ok: self.populate_devices_ok,
        }
    }
}

thread_local! {
    static IO_STATE: RefCell<IoState> = RefCell::new(IoState::new());
}
