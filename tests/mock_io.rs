use std::cell::RefCell;
use pitchgrid_continuum::global::DeviceType;
use pitchgrid_continuum::midi_ports::{IIo, IoDevice};

pub fn input_state() -> IoState {
    INPUT_STATE.with(|s| s.borrow().clone())
}

pub fn output_state() -> IoState {
    OUTPUT_STATE.with(|s| s.borrow().clone())
}

pub struct MockIo {
    device_type: DeviceType,
    /// Controls the return value of `device()`. Set directly on the mock to configure.
    device: Option<MockDevice>,
}

impl MockIo {
    pub fn new(device_type: DeviceType, device_names: Vec<String>) -> Self {
        let mut state = IoState::new();
        state.device_names = device_names;
        match device_type {
            DeviceType::Input => INPUT_STATE.replace(state),
            DeviceType::Output => OUTPUT_STATE.replace(state),
        };
        MockIo { device_type, device: None }
    }

    pub fn state(&self) -> IoState {
        match self.device_type {
            DeviceType::Input => input_state(),
            DeviceType::Output => output_state(),
        }
    }

    /// If the specified device name can be found in the device_names,
    /// sets the device to a MockDevice with the `name` as specified
    /// and `index` set to the index of the name in the device_names.
    /// Otherwise sets the device to None.
    pub fn set_device(&mut self, name: &str) {
        self.device = {
            let device_names = self.state().device_names;
            device_names.iter().position(|n| n == name).map(|index| MockDevice {
                index,
                name: name.to_string(),
            })
        };
        let device_clone = self.device.clone();
        match self.device_type {
            DeviceType::Input => INPUT_STATE.with_borrow_mut(|s| s.device = device_clone),
            DeviceType::Output => OUTPUT_STATE.with_borrow_mut(|s| s.device = device_clone),
        }
    }
}

impl IIo for MockIo {
    fn device(&self) -> Option<&dyn IoDevice> {
        self.device.as_ref().map(|d| d as &dyn IoDevice)
    }

    fn device_names(&self) -> Vec<String> {
        self.state().device_names
    }

    fn populate_devices(&mut self, persisted_device_name: &str) {
        match self.device_type {
            DeviceType::Input => INPUT_STATE.with_borrow_mut(|s| {
                s.populate_devices_persisted_device_name = Some(persisted_device_name.to_string());
            }),
            DeviceType::Output => OUTPUT_STATE.with_borrow_mut(|s| {
                s.populate_devices_persisted_device_name = Some(persisted_device_name.to_string());
            }),
        };
    }
}

#[derive(Clone)]
pub struct MockDevice {
    index: usize,
    name: String,
}

impl IoDevice for MockDevice {
    fn index(&self) -> usize { self.index }
    fn name(&self) -> String { self.name.clone() }
}

pub struct IoState {
    pub device: Option<MockDevice>,
    pub device_names: Vec<String>,
    pub populate_devices_persisted_device_name: Option<String>,
}

impl IoState {
    pub fn device_name(&self) -> Option<String> {
        self.device.as_ref().map(|d| d.name.clone())
    }

    pub fn device_index(&self) -> Option<usize> {
        self.device.as_ref().map(|d| d.index)
    }

    pub fn new() -> Self {
        IoState {
            device: None,
            device_names: vec![],
            populate_devices_persisted_device_name: None,
        }
    }
}

impl Clone for IoState {
    fn clone(&self) -> Self {
        IoState {
            device: self.device.clone(),
            device_names: self.device_names.clone(),
            populate_devices_persisted_device_name: self.populate_devices_persisted_device_name.clone(),
        }
    }
}

thread_local! {
    static INPUT_STATE: RefCell<IoState> = RefCell::new(IoState::new());
    static OUTPUT_STATE: RefCell<IoState> = RefCell::new(IoState::new());
}
