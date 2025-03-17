use std::{env, path::PathBuf};

use anyhow::{Ok, Result};

pub struct FileSystem;

impl FileSystem {
    pub fn new() -> Self {
        Self
    }

    pub fn get_executable_directory(&self) -> Result<PathBuf> {
        let executable_path = if cfg!(test) {
            env::current_exe()?.parent().unwrap().to_path_buf()
        } else {
            env::current_exe()?
        };

        let executable_directory = executable_path.parent().unwrap();
        let path = executable_directory.to_path_buf();
        
        Ok(path)
    }

}
