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
    pub fn new(device_names: Vec<String>) -> Self {
        IO_STATE.replace(IoState::new());
        IO_STATE.with_borrow_mut(|s| {
            s.device_names = device_names.clone();
        });
        MockIo { device: None }
    }

    /// If the specified device name can be found in IO_STATE.device_names,
    /// sets the device to a MockDevice with the `name` as specified
    /// and `index` set to the index of the name in IO_STATE.device_names.
    /// Otherwise sets the device to None.
    pub fn set_device(&mut self, name: &str) {
        self.device = IO_STATE.with(|s| {
            let s = s.borrow();
            s.device_names.iter().position(|n| n == name).map(|index| MockDevice {
                index,
                name: name.to_string(),
            })
        });
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
        IO_STATE.with(|s| s.borrow().device_names.clone())
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

    pub device_names: Vec<String>,

    pub populate_devices_count: u16,
    pub populate_devices_persisted_device_name: Option<String>,
    pub populate_devices_ok: bool,
}

impl IoState {
    pub fn new() -> Self {
        IoState {
            device_count: 0,
            device_names: Vec::new(),
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
            device_names: self.device_names.clone(),

            populate_devices_count: self.populate_devices_count,
            populate_devices_persisted_device_name: self.populate_devices_persisted_device_name.clone(),
            populate_devices_ok: self.populate_devices_ok,
        }
    }
}

thread_local! {
    static IO_STATE: RefCell<IoState> = RefCell::new(IoState::new());
}
