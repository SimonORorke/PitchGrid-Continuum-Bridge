use version_compare::Version;
use app_info::VERSION;
use crate::release_info::SharedReleaseInfo;

/// A utility for checking whether a new version of the application is
/// available.
pub struct VersionChecker {
    release_info: SharedReleaseInfo,
}

impl VersionChecker {
    pub fn new(release_info: SharedReleaseInfo) -> Self {
        VersionChecker {
            release_info
        }
    }

    /// Checks whether a new version of the application is available.
    /// Returns the new version, if one is available, and it's not a version we are ignoring,
    /// otherwise `None`.
    /// `ignore_version`: If specified, versions less than or equal to this version will be ignored.
    pub fn check_for_new_version(&self, ignore_version: &str) -> Option<String> {
        let current_version = Version::from(VERSION)?;

        let release_info = self.release_info.lock().unwrap();
        let latest_version_string = release_info.get_latest_version_for_platform()?;
        let latest_version = Version::from(&latest_version_string)?;

        if latest_version <= current_version {
            return None;
        }

        if let Some(ignore_version) = Version::from(ignore_version) {
            if latest_version <= ignore_version {
                return None;
            }
        }

        Some(latest_version_string)
    }
}