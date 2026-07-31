use std::sync::{Arc, Mutex};
use version_compare::Version;

pub trait IReleaseInfo {
    /// Returns the version of the latest release of the application available on GitHub for the
    /// current platform.
    fn get_latest_version_for_platform(&self) -> Option<Version<'_>>;
}

pub struct ReleaseInfo {}

impl ReleaseInfo {
    pub fn new() -> Self {
        ReleaseInfo {}
    }
}

impl IReleaseInfo for ReleaseInfo {
    fn get_latest_version_for_platform(&self) -> Option<Version<'_>> {
        todo!()
        // The list of releases, latest first, can be found at the link specified by
        // app_info::RELEASES_LINK.
        // The version string from which the returned `Version` is to be constructed
        // is the release tag less the "v" prefix.
        // For example, the version string for the release tagged "v1.2.3" is "1.2.3".
        // Platforms to be supported: 64-bit Windows, macOS.
        // I don't know whether the target platform can be determined via the GitHub API.
        // If not, it can be determined from the release name, which will end with
        // " (64-bit Windows)" for 64-bit Windows or " (macOS)" for macOS.
        // `None` is returned
        // if the target platform cannot be determined
        // or the version cannot be parsed
        // or there are no releases available for the current platform.
    }
}

pub type SharedReleaseInfo = Arc<Mutex<Box<dyn IReleaseInfo + Send>>>;
