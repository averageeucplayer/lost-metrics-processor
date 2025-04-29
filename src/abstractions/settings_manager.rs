use std::{fs::{self, File}, io::Write, path::PathBuf};
use anyhow::*;
use log::debug;
use lost_metrics_core::models::Settings;

#[cfg(test)]
use mockall::automock;

use super::FileSystem;

#[cfg_attr(test, automock)]
pub trait SettingsManager {
    fn get_or_create(&mut self) -> Result<Settings>;
    fn write(&mut self, settings: &Settings) -> Result<()>;
}

pub struct DefaultSettingsManager<'a, FS: FileSystem> {
    file_system: &'a mut FS,
    path: PathBuf
}

impl<'a, FS: FileSystem> SettingsManager for DefaultSettingsManager<'a, FS> {
    fn get_or_create(&mut self) -> Result<Settings> {

        if self.file_system.exists(&self.path) {
            let file = self.file_system.get_reader(&self.path)?;
            let settings = serde_json::from_reader(file)?;
            return Ok(settings);
        }

        let file = self.file_system.get_writer(&self.path)?;
        let settings = Settings::default();
        serde_json::to_writer(file, &settings)?;

        Ok(settings)
    }

    fn write(&mut self, settings: &Settings) -> Result<()> {
        let mut file = self.file_system.get_writer(&self.path)?;
        let json_str = serde_json::to_string_pretty(&settings)?;
        let bytes = json_str.as_bytes();

        file.write_all(bytes)?;

        Ok(())
    }
}

impl<'a, FS: FileSystem> DefaultSettingsManager<'a, FS> {
    pub fn new(file_system: &'a mut FS, path: PathBuf) -> Self {
        Self { file_system, path }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

    use lost_metrics_core::models::Settings;

    use crate::abstractions::MemoryFileSystem;

    use super::{DefaultSettingsManager, SettingsManager};

    fn get_semi_random_settings_path() -> PathBuf {
        let path = env::current_dir().unwrap();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        path.join(format!("settings_{}.json", timestamp))
    }

    #[test]
    fn should_create_settings() {
        let path = get_semi_random_settings_path();

        let mut file_system = MemoryFileSystem::new();
        let mut settings_manager = DefaultSettingsManager::new(&mut file_system, path.clone());
        let settings = settings_manager.get_or_create().unwrap();

        assert_eq!(settings.general.boss_only_damage, false);
    }

    #[test]
    fn should_save_settings() {
        let path = get_semi_random_settings_path();

        let mut file_system = MemoryFileSystem::new();
        let mut settings_manager = DefaultSettingsManager::new(&mut file_system, path.clone());
        let mut settings = Settings::default();
        settings.general.hide_names = true;

        settings_manager.write(&settings).unwrap();
        let settings = settings_manager.get_or_create().unwrap();

        assert_eq!(settings.general.hide_names, true);
    }
}