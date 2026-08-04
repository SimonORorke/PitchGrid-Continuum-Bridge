use std::sync::{LazyLock, Mutex, MutexGuard};
use pitchgrid_continuum::release_info::IReleaseInfo;

pub fn mock_release_info() -> MutexGuard<'static, MockReleaseInfo> {
    MOCK_RELEASE_INFO.lock().unwrap_or_else(|e| e.into_inner())
}

pub static MOCK_RELEASE_INFO: LazyLock<Mutex<MockReleaseInfo>> =
    LazyLock::new(|| Mutex::new(MockReleaseInfo::new_state()));

pub struct MockReleaseInfo {
    pub get_latest_version_for_platform_count: u16,
    pub latest_version: Option<String>,
}

impl MockReleaseInfo {
    pub fn new_state() -> Self {
        Self {
            get_latest_version_for_platform_count: 0,
            latest_version: None,
        }
    }

    pub fn new() -> Self {
        *MOCK_RELEASE_INFO.lock().unwrap_or_else(|e|
            e.into_inner()) = MockReleaseInfo::new_state();
        MockReleaseInfo::new_state()
    }

    pub fn simulate_latest_version(value: Option<String>) {
        MOCK_RELEASE_INFO.lock().unwrap_or_else(|e|
            e.into_inner()).latest_version = value;
    }
}

impl IReleaseInfo for MockReleaseInfo {
    fn get_latest_version_for_platform(&self) -> Option<String> {
        let mut state = MOCK_RELEASE_INFO.lock().unwrap_or_else(
            |e| e.into_inner());
        state.get_latest_version_for_platform_count += 1;
        state.latest_version.clone()
    }
}

