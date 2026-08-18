use std::sync::{Arc, Mutex};
use serde::Deserialize;

/// Model for information on releases of this application.
pub trait IReleaseInfo {
    /// Returns the version string of the latest release of the application available on GitHub for
    /// the current platform.
    fn get_latest_version_for_platform(&self) -> Option<String>;
}

/// Information on releases of this application.
pub struct ReleaseInfo {}

/// Info on an application release held on GitHub.
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
}

impl ReleaseInfo {
    pub fn new() -> Self {
        ReleaseInfo {}
    }

    /// Returns the release name suffix from which the release's target platform may be determined.
    /// Platforms to be supported: 64-bit Windows, macOS.
    fn current_platform_release_name_suffix() -> Option<&'static str> {
        if cfg!(all(target_os = "windows", target_pointer_width = "64")) {
            Some(" (64-bit Windows)")
        } else if cfg!(target_os = "macos") {
            Some(" (macOS)")
        } else {
            None
        }
    }

    fn github_releases_api_url() -> Option<String> {
        app_info::RELEASES_LINK
            .strip_prefix("https://github.com/")
            .and_then(|path| path.strip_suffix("/releases"))
            .map(|repo_path| format!("https://api.github.com/repos/{repo_path}/releases"))
    }
}

impl IReleaseInfo for ReleaseInfo {
    fn get_latest_version_for_platform(&self) -> Option<String> {
        let platform_suffix = Self::current_platform_release_name_suffix()?;
        // Get the list of the application's releases, latest first, from GitHub.
        let releases_api_url = Self::github_releases_api_url()?;
        let response_body = ureq::get(&releases_api_url)
            .header("User-Agent", app_info::APP_TITLE)
            .call()
            .ok()?
            .body_mut()
            .read_to_string()
            .ok()?;
        let releases: Vec<GitHubRelease> = serde_json::from_str(&response_body).ok()?;
        // Find the latest release for the current platform.
        for release in releases {
            let release_name = release.name.as_deref()?;
            if !release_name.ends_with(platform_suffix) {
                continue;
            }
            // The version string from which the returned `Version` is to be constructed
            // is the release tag less the "v" prefix.
            let version = release.tag_name.strip_prefix('v')?;
            return Some(version.to_string());
        }
        // No release for the current platform was found.
        None
    }
}

pub type SharedReleaseInfo = Arc<Mutex<Box<dyn IReleaseInfo + Send>>>;