use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use pitchgrid_continuum::path_finder::PathFinder;

#[derive(Clone, Debug)]
pub struct TempPathFinder {
    config_folder: Arc<TempDir>,
}

impl TempPathFinder {
    pub fn new() -> Self {
        TempPathFinder {
            config_folder: Arc::new(TempDir::new().expect(
                "Failed to create temporary directory")),
        }
    }
}


impl PathFinder for TempPathFinder {
    fn config_folder_path(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(self.config_folder.path().to_path_buf())
    }
}

