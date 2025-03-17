use std::path::PathBuf;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait RegionStore {
    fn get(&self) -> Option<String>;
    fn get_path(&self) -> String;
}

pub struct DefaultRegionStore {
    path: PathBuf
}

impl RegionStore for DefaultRegionStore {
    fn get(&self) -> Option<String> {
        match std::fs::read_to_string(&self.path) {
            std::result::Result::Ok(region) => {
                return Some(region)
            }
            Err(_) => {
                // warn!("failed to read region file. {}", e);
            }
        }

        None
    }
    
    fn get_path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

impl DefaultRegionStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

