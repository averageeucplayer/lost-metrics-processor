use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}, time::Duration};
use crate::{constants::LOW_PERFORMANCE_MODE_DURATION, live::{self, trackers::Trackers}};
use crate::live::{abstractions::{file_system::FileSystem, *}, encounter_state::EncounterState, flags::{AtomicBoolFlags, Flags}, heartbeat_api::DefaultHeartbeatApi, packet_handler::DefaultPacketHandler, stats_api::DefaultStatsApi, StartOptions};
use log::{error, info};
use anyhow::{Ok, Result};
use lost_metrics_store::{connection_pool, encounter_service::{self, DefaultEncounterService}, migration_runner::MigrationRunner, repository::SqliteRepository};
use tokio::sync::Mutex;

pub struct BackgroundWorker {
    flags: Arc<AtomicBoolFlags>,
    handle: Option<std::thread::JoinHandle<std::result::Result<(), ()>>>
}

impl BackgroundWorker {
    pub fn new() -> Self {
        let flags = Arc::new(AtomicBoolFlags::new());

        Self {
            flags,
            handle: None
        }
    }

    pub fn start(&mut self) {
        let flags = self.flags.clone();

        let handle = std::thread::spawn(|| {

            let file_system = DefaultFileSystem::new();
            let executable_directory = file_system.get_executable_directory().unwrap();
            let settings_path = executable_directory.join("settings.json").clone();
            let mut settings_manager = DefaultSettingsManager::new(file_system, settings_path);
    
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
    
           
            let settings = settings_manager.get_or_create().unwrap();
    
            if settings.general.boss_only_damage {
                flags.set_boss_only_damage(true);
                info!("boss only damage enabled")
            }
    
            if settings.general.low_performance_mode {
                options.duration = LOW_PERFORMANCE_MODE_DURATION;
                info!("low performance mode enabled")
            }
    
           
            let event_emitter = Arc::new(DefaultEventEmitter::new());
            let event_listener = Arc::new(DefaultEventListener::new());
            let region_store = Arc::new(DefaultRegionStore::new(options.region_path.clone()));
            let local_player_store = Arc::new(RwLock::new(DefaulLocalPlayerStore::new(options.local_player_path.clone())));
    
            let connection_pool = connection_pool::get(&options.database_path);
            let migration_runner = MigrationRunner::new(connection_pool.clone());
            let repository = SqliteRepository::new(connection_pool.clone());
            let encounter_service = Arc::new(DefaultEncounterService::new(repository));
    
            match migration_runner.run() {
                Err(err) => {
                    error!("Fatal: {}", err);
                },
                _ => {}
            };
           
            let heartbeat_api = Arc::new(Mutex::new(DefaultHeartbeatApi::new()));
            let stats_api = Arc::new(Mutex::new(DefaultStatsApi::new(options.version.clone())));
            let packet_sniffer = PacketSnifferStub::new();
            let trackers = Rc::new(RefCell::new(Trackers::new()));
    
            let mut packet_handler = DefaultPacketHandler::new(
                flags.clone(),
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
                &mut state,
                options,
                event_emitter,
                event_listener,
                region_store,
                local_player_store,
                encounter_service,
                heartbeat_api,
                stats_api)
                .map_err(|e| {
                    error!("unexpected error occurred in parser: {}", e);
                })
        });

        self.handle = Some(handle);
    }

    pub fn join(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join().map_err(|err| anyhow::anyhow!("Error while stopping processor: {:?}", err))?;
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.flags.set_stop();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join().map_err(|err| anyhow::anyhow!("Error while stopping processor: {:?}", err))?;
        }

        Ok(())
    }
}