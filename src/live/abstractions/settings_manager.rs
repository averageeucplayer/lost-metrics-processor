use std::{fs::File, io::Write, path::PathBuf};
use anyhow::*;
use log::debug;
use lost_metrics_core::models::Settings;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait SettingsManager {
    fn get_or_create(&self) -> Result<Settings>;
    fn write(&self, settings: &Settings) -> Result<()>;
}

pub struct DefaultSettingsManager {
    path: PathBuf
}

impl SettingsManager for DefaultSettingsManager {
    fn get_or_create(&self) -> Result<Settings> {

        if self.path.exists() {
            let file = File::open(&self.path)?;
            let settings = serde_json::from_reader(&file)?;
            return Ok(settings);
        }

        let file = File::create(&self.path)?;
        let settings = Settings::default();
        serde_json::to_writer(file, &settings)?;

        Ok(settings)
    }

    fn write(&self, settings: &Settings) -> Result<()> {
        let mut file = File::create(&self.path)?;
        let json_str = serde_json::to_string_pretty(&settings)?;
        let bytes = json_str.as_bytes();

        file.write_all(bytes)?;

        Ok(())
    }
}

impl DefaultSettingsManager {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

    use lost_metrics_core::models::Settings;

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

        let settings_manager = DefaultSettingsManager::new(path.clone());
        let settings = settings_manager.get_or_create().unwrap();

        assert_eq!(settings.general.boss_only_damage, false);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn should_save_settings() {
        let path = get_semi_random_settings_path();

        let settings_manager = DefaultSettingsManager::new(path.clone());
        let mut settings = Settings::default();
        settings.general.hide_names = true;

        settings_manager.write(&settings).unwrap();
        let settings = settings_manager.get_or_create().unwrap();

        assert_eq!(settings.general.hide_names, true);

        std::fs::remove_file(path).unwrap();
    }
}