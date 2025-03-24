use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::{Arc, RwLock}, thread::JoinHandle, time::Duration};
use crate::{constants::LOW_PERFORMANCE_MODE_DURATION, live::{self, trackers::Trackers}};
use crate::live::{abstractions::{file_system::FileSystem, *}, encounter_state::EncounterState, flags::{AtomicBoolFlags, Flags}, heartbeat_api::DefaultHeartbeatApi, packet_handler::DefaultPacketHandler, stats_api::DefaultStatsApi, StartOptions};
use log::{error, info};
use anyhow::{Ok, Result};
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandler;
use lost_metrics_store::{connection_pool, encounter_service::{self, DefaultEncounterService}, migration_runner::MigrationRunner, repository::SqliteRepository};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::sync::Mutex;

pub struct BackgroundWorker {
    flags: Arc<AtomicBoolFlags>,
    file_system: DefaultFileSystem,
    connection_pool: Option<Pool<SqliteConnectionManager>>,
    executable_directory: PathBuf,
    handle: Option<JoinHandle<Result<()>>>
}

impl BackgroundWorker {
    pub fn new() -> Self {
        let flags = Arc::new(AtomicBoolFlags::new());
        let file_system = DefaultFileSystem::new();
        let executable_directory = file_system.get_executable_directory().unwrap();

        Self {
            flags,
            file_system,
            connection_pool: None,
            executable_directory,
            handle: None
        }
    }

    pub fn create_default_options(&self) -> StartOptions {
        let executable_directory = self.executable_directory.clone();

        let mut options = StartOptions {
            version: env!("CARGO_PKG_VERSION").to_string(),
            port: 6040,
            region_path: executable_directory.join("current_region").clone(),
            local_player_path: executable_directory.join("local_players.json").clone(),
            database_path: executable_directory.join("encounters.db").clone(),
            raid_end_capture_timeout: Duration::from_secs(10),
            duration: Duration::from_millis(500),
            party_duration: Duration::from_millis(2000)
        };

        options
    }

    pub fn apply_settings(&mut self, options: &mut StartOptions) {
        let executable_directory = self.executable_directory.clone();
        let settings_path = executable_directory.join("settings.json").clone();
        let mut settings_manager = DefaultSettingsManager::new(&mut self.file_system, settings_path);

        let settings = settings_manager.get_or_create().unwrap();

        if settings.general.boss_only_damage {
            self.flags.set_boss_only_damage(true);
            info!("boss only damage enabled")
        }

        if settings.general.low_performance_mode {
            options.duration = LOW_PERFORMANCE_MODE_DURATION;
            info!("low performance mode enabled")
        }
    }

    pub fn start(&mut self, options: StartOptions) {
        let connection_pool = self.connection_pool.clone().expect("Should run migrations first");
        let flags = self.flags.clone();
        let handle = std::thread::spawn(move || Self::start_inner(flags, options, connection_pool));

        self.handle = Some(handle);
    }

    pub fn run_migrations(&mut self, options: &StartOptions) -> Result<()> {
        let connection_pool = connection_pool::get(&options.database_path);
        self.connection_pool = Some(connection_pool.clone());
        let migration_runner = MigrationRunner::new(connection_pool.clone());

        migration_runner.run()?;

        Ok(())
    }

    fn start_inner(flags: Arc<AtomicBoolFlags>, options: StartOptions, connection_pool: Pool<SqliteConnectionManager>) -> Result<()> {
        let event_emitter = Arc::new(DefaultEventEmitter::new());
        let event_listener = Arc::new(DefaultEventListener::new());
        let region_store = Arc::new(DefaultRegionStore::new(options.region_path.clone()));
        let local_player_store = Arc::new(RwLock::new(DefaulLocalPlayerStore::new(options.local_player_path.clone())));
        let repository = SqliteRepository::new(connection_pool);
        let encounter_service = Arc::new(DefaultEncounterService::new(repository));
        let heartbeat_api = Arc::new(Mutex::new(DefaultHeartbeatApi::new()));
        let stats_api = Arc::new(Mutex::new(DefaultStatsApi::new(options.version.clone())));
        let packet_sniffer = PacketSnifferStub::new();
        let trackers = Rc::new(RefCell::new(Trackers::new()));

        let damage_encryption_handler = Arc::new(DamageEncryptionHandler::new());
        let mut packet_handler = DefaultPacketHandler::new(
            flags.clone(),
            damage_encryption_handler.clone(),
            trackers.clone(),
            local_player_store.clone(),
            event_emitter.clone(),
            region_store.clone(),
            encounter_service.clone(),
            stats_api.clone(),
        );
            
        let mut state = EncounterState::new(
            trackers,
            options.version.clone());

        live::start(
            flags,
            packet_sniffer,
            &mut packet_handler,
            damage_encryption_handler,
            &mut state,
            options,
            event_emitter,
            event_listener,
            region_store,
            local_player_store,
            encounter_service,
            heartbeat_api,
            stats_api)?;

        Ok(())
    }

    pub fn join(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.join()
                .map_err(|err| anyhow::anyhow!("Error while stopping processor: {:?}", err))?.unwrap();
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.flags.set_stop();
        self.join()?;

        Ok(())
    }
}