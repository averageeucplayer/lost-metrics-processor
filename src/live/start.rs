
use crate::live::encounter_state::EncounterState;
use crate::live::stats_api::StatsApi;
use super::flags::Flags;
use super::packet_handler::PacketHandler;
use super::utils::send_to_ui;
use super::{abstractions::*, register_listeners};
use super::heartbeat_api::HeartbeatApi;
use anyhow::Result;
use hashbrown::HashMap;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_store::encounter_service::EncounterService;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub struct StartOptions {
    pub version: String,
    pub port: u16,
    pub region_path: PathBuf,
    pub local_player_path: PathBuf,
    pub database_path: PathBuf,
    pub raid_end_capture_timeout: Duration,
    pub party_duration: Duration,
    pub duration: Duration
}

pub fn start<FL, PS, PH, DH, EE, EL, RS, LP, ES, HB, SA>(
    flags: Arc<FL>,
    packet_sniffer: PS,
    packet_handler: &mut PH,
    damage_encryption_handler: Arc<DH>, 
    state: &mut EncounterState,
    options: StartOptions,
    event_emitter: Arc<EE>,
    event_listener: Arc<EL>,
    region_store: Arc<RS>,
    local_player_store: Arc<RwLock<LP>>,
    encounter_service: Arc<ES>,
    heartbeat_api: Arc<Mutex<HB>>,
    stats_api: Arc<Mutex<SA>>) 
    -> Result<()> 
    where
        FL: Flags,
        PS: PacketSniffer,
        PH: PacketHandler,
        DH: DamageEncryptionHandlerTrait,
        EE: EventEmitter,
        EL: EventListener,
        RS: RegionStore,
        LP: LocalPlayerStore,
        ES: EncounterService,
        HB: HeartbeatApi,
        SA: StatsApi
    {
    let rt: Handle = Handle::current();
    // let settings = options.settings;
    let version = options.version.clone();
    let port = options.port;
    
    //warn!("Error starting capture: {}", e);
    let rx = packet_sniffer.start_capture(port, region_store.get_path())?;

    damage_encryption_handler.start()?;
    
    {
        let mut local_player_store = local_player_store.write().unwrap();
        let is_local_loaded = local_player_store.load()?;

        if is_local_loaded {
            state.client_id = Some(local_player_store.get().client_id);
        }
    }

    state.region = region_store.get();

    register_listeners(
        event_emitter.clone(),
        event_listener.clone(),
        flags.clone()
    );

    while let Ok((op, data)) = rx.recv() {
        let now = Instant::now();
        
        if flags.triggered_stop() {
            return Ok(());
        }

        if flags.triggered_reset() {
            state.soft_reset(true);
            flags.clear_reset();
        }
        
        if flags.triggered_pause() {
            continue;
        }

        if flags.triggered_save() {
            flags.reset_save();
            
            state.party_info = state.get_party_from_tracker();
            state.save_to_db(state.client_id, stats_api.clone(), true, encounter_service.clone(), event_emitter.clone());
            state.saved = true;
            state.resetting = true;
        }

        if flags.triggered_boss_only_damage() {
            state.boss_only_damage = true;
        } else {
            state.boss_only_damage = false;
            state.encounter.boss_only_damage = false;
        }

        match packet_handler.handle(op, &data, state, &options, rt.clone()) {
            Err(_) => {

            },
            _ => {}
        }

        if state.last_update.elapsed() >= options.duration || state.resetting || state.boss_dead_update {
            state.last_update = send_to_ui(state, event_emitter.clone(), &options);
        }

        if state.resetting {
            state.soft_reset(true);
            state.resetting = false;
            state.saved = false;
            state.party_freeze = false;
            state.party_cache = None;
            state.party_map_cache = HashMap::new();
        }

        {
            let local_player_store = local_player_store.read().unwrap();
            let client_id = local_player_store.get().client_id.clone();
            let version = version.clone();
            let heartbeat_api = heartbeat_api.clone();
            if let Some(region) = state.region.clone() {
                tokio::task::spawn(async move {
                    let mut heartbeat_api = heartbeat_api.lock().await;
        
                    if heartbeat_api.can_send() {
                        heartbeat_api.send(client_id, version, region).await;
            
                        heartbeat_api.refresh();
                    }
                });   
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};
    use chrono::Utc;
    use lost_metrics_core::models::Encounter;
    use mockall::predicate::always;
    use mockall::mock;
    use crate::live::{flags::MockFlags, heartbeat_api::MockHeartbeatApi, stats_api::MockStatsApi, test_utils::*, trackers::Trackers};
    use crate::live::test_utils::MockEncounterService;
    use super::*;

    // #[tokio::test]
    // async fn should_handle_packet() {

    //     let options = create_start_options();
    //     let flags = create_and_setup_flags();
    //     let packet_sniffer = create_and_setup_packet_sniffer();
    //     let mut packet_handler = create_and_setup_packet_handler();

    //     let trackers = Rc::new(RefCell::new(Trackers::new()));
    //     let mut state = EncounterState::new(trackers, options.version.clone());

    //     let event_emitter = MockEventEmitter::new();
    //     let event_emitter = Arc::new(event_emitter);

    //     let event_listener = create_and_setup_event_listener();
    //     let region_store = create_and_setup_region_store();
    //     let local_player_store = create_and_setup_local_player_store();

    //     let repository = MockEncounterService::new();
    //     let repository = Arc::new(repository);
        
    //     let heartbeat_api = MockHeartbeatApi::new();
    //     let heartbeat_api = Arc::new(Mutex::new(heartbeat_api));
        
    //     let stats_api = MockStatsApi::new();
    //     let stats_api = Arc::new(Mutex::new(stats_api));

    //     let damage_encryption_handler = MockDamageEncryptionHandlerTrait::new();

    //     start(
    //         flags,
    //         packet_sniffer,
    //         &mut packet_handler,
    //         damage_encryption_handler,
    //         &mut state,
    //         options,
    //         event_emitter,
    //         event_listener, 
    //         region_store, 
    //         local_player_store,
    //         repository,
    //         heartbeat_api,
    //         stats_api
    //     ).unwrap();
    // }

    // #[tokio::test]
    // async fn should_reset_requested_from_ui() {
    //     let options = create_start_options();
    //     let mut flags = MockFlags::new();

    //     flags
    //         .expect_triggered_stop()
    //         .returning(|| false);

    //     flags
    //         .expect_triggered_reset()
    //         .returning(|| true);

    //     flags
    //         .expect_clear_reset()
    //         .return_once(|| {});

    //     flags
    //         .expect_triggered_pause()
    //         .returning(|| false);

    //     flags
    //         .expect_triggered_save()
    //         .returning(|| false);

    //     flags
    //         .expect_triggered_boss_only_damage()
    //         .returning(|| false);

    //     let flags = Arc::new(flags);

    //     let packet_sniffer = create_and_setup_packet_sniffer();
    //     let mut packet_handler = create_and_setup_packet_handler();

    //     let trackers = Rc::new(RefCell::new(Trackers::new()));
    //     let mut state = EncounterState::new(trackers, options.version.clone());

    //     let event_emitter = MockEventEmitter::new();
    //     let event_emitter = Arc::new(event_emitter);

    //     let event_listener = create_and_setup_event_listener();
    //     let region_store = create_and_setup_region_store();
    //     let local_player_store = create_and_setup_local_player_store();

    //     let repository = MockEncounterService::new();
    //     let repository = Arc::new(repository);
        
    //     let heartbeat_api = MockHeartbeatApi::new();
    //     let heartbeat_api = Arc::new(Mutex::new(heartbeat_api));
        
    //     let stats_api = MockStatsApi::new();
    //     let stats_api = Arc::new(Mutex::new(stats_api));

    //     start(
    //         flags,
    //         packet_sniffer,
    //         &mut packet_handler,
    //         &mut state,
    //         options,
    //         event_emitter,
    //         event_listener, 
    //         region_store, 
    //         local_player_store,
    //         repository,
    //         heartbeat_api,
    //         stats_api
    //     ).unwrap();
    // }

    // #[tokio::test]
    // async fn should_reset_on_flag() {
    //     let options = create_start_options();
    //     let flags = create_and_setup_flags();
    //     let packet_sniffer = create_and_setup_packet_sniffer();
    //     let mut packet_handler = create_and_setup_packet_handler();

    //     let trackers = Rc::new(RefCell::new(Trackers::new()));
    //     let mut state = EncounterState::new(trackers, options.version.clone());
    //     state.resetting = true;
    //     state.saved = true;
    //     state.party_freeze = true;
    //     state.party_cache = Some(vec![]);

    //     let event_emitter = MockEventEmitter::new();
    //     let event_emitter = Arc::new(event_emitter);

    //     let event_listener = create_and_setup_event_listener();
    //     let region_store = create_and_setup_region_store();
    //     let local_player_store = create_and_setup_local_player_store();

    //     let repository = MockEncounterService::new();
    //     let repository = Arc::new(repository);
        
    //     let heartbeat_api = MockHeartbeatApi::new();
    //     let heartbeat_api = Arc::new(Mutex::new(heartbeat_api));
        
    //     let stats_api = MockStatsApi::new();
    //     let stats_api = Arc::new(Mutex::new(stats_api));

    //     start(
    //         flags,
    //         packet_sniffer,
    //         &mut packet_handler,
    //         &mut state,
    //         options,
    //         event_emitter,
    //         event_listener, 
    //         region_store, 
    //         local_player_store,
    //         repository,
    //         heartbeat_api,
    //         stats_api
    //     ).unwrap();

    //     assert_eq!(state.resetting, false);
    //     assert_eq!(state.saved, false);
    //     assert_eq!(state.party_freeze, false);
    //     assert_eq!(state.party_cache, None);
    //     assert!(state.party_map_cache.is_empty());
    // }

    // #[tokio::test]
    // async fn should_save_to_db() {
    //     let options = create_start_options();
    //     let mut flags = MockFlags::new();

    //     flags
    //         .expect_triggered_stop()
    //         .returning(|| false);

    //     flags
    //         .expect_triggered_reset()
    //         .returning(|| false);

    //     flags
    //         .expect_clear_reset()
    //         .return_once(|| {});

    //     flags
    //         .expect_triggered_pause()
    //         .returning(|| false);

    //     flags
    //         .expect_triggered_save()
    //         .returning(|| true);

    //     flags
    //         .expect_reset_save()
    //         .returning(|| {});

    //     flags
    //         .expect_triggered_boss_only_damage()
    //         .returning(|| false);

    //     let flags = Arc::new(flags);

    //     let packet_sniffer = create_and_setup_packet_sniffer();
    //     let mut packet_handler = create_and_setup_packet_handler();

    //     let trackers = Rc::new(RefCell::new(Trackers::new()));
    //     let mut state = EncounterState::new(trackers, options.version.clone());

    //     update_state_with_player_and_boss(&mut state);

    //     let event_emitter = MockEventEmitter::new();
    //     let event_emitter = Arc::new(event_emitter);

    //     let event_listener = create_and_setup_event_listener();
    //     let region_store = create_and_setup_region_store();
    //     let local_player_store = create_and_setup_local_player_store();

    //     let repository = MockEncounterService::new();
    //     let repository = Arc::new(repository);
        
    //     let heartbeat_api = MockHeartbeatApi::new();
    //     let heartbeat_api = Arc::new(Mutex::new(heartbeat_api));
        
    //     let stats_api = MockStatsApi::new();
    //     let stats_api = Arc::new(Mutex::new(stats_api));

    //     start(
    //         flags,
    //         packet_sniffer,
    //         &mut packet_handler,
    //         &mut state,
    //         options,
    //         event_emitter,
    //         event_listener, 
    //         region_store, 
    //         local_player_store,
    //         repository,
    //         heartbeat_api,
    //         stats_api
    //     ).unwrap();
    // }

    // #[tokio::test]
    // async fn should_send_party_to_ui() {

    //     let options = create_start_options();
    //     let flags = create_and_setup_flags();
    //     let packet_sniffer = create_and_setup_packet_sniffer();
    //     let mut packet_handler = create_and_setup_packet_handler();

    //     let trackers = Rc::new(RefCell::new(Trackers::new()));
    //     let mut state = EncounterState::new(trackers, options.version.clone());

    //     let entity = create_player_stats();
    //     state.encounter.entities.insert(entity.name.clone(), entity);
    //     state.last_update = Instant::now() + options.duration;
    //     state.last_party_update = Instant::now() + options.party_duration;

    //     let mut event_emitter = MockEventEmitter::new();

    //     event_emitter
    //         .expect_emit()
    //         .with(always(), always())
    //         .returning(|_, _: Option<Encounter>| anyhow::Ok(()));

    //     let event_emitter = Arc::new(event_emitter);

    //     let event_listener = create_and_setup_event_listener();
    //     let region_store = create_and_setup_region_store();
    //     let local_player_store = create_and_setup_local_player_store();

    //     let repository = MockEncounterService::new();
    //     let repository = Arc::new(repository);
        
    //     let heartbeat_api = MockHeartbeatApi::new();
    //     let heartbeat_api = Arc::new(Mutex::new(heartbeat_api));
        
    //     let stats_api = MockStatsApi::new();
    //     let stats_api = Arc::new(Mutex::new(stats_api));

    //     start(
    //         flags,
    //         packet_sniffer,
    //         &mut packet_handler,
    //         &mut state,
    //         options,
    //         event_emitter,
    //         event_listener, 
    //         region_store, 
    //         local_player_store,
    //         repository,
    //         heartbeat_api,
    //         stats_api
    //     ).unwrap();
    // }

    // #[tokio::test]
    // async fn should_send_encounter_to_ui() {

    //     let options = create_start_options();
    //     let flags = create_and_setup_flags();
    //     let packet_sniffer = create_and_setup_packet_sniffer();
    //     let mut packet_handler = create_and_setup_packet_handler();

    //     let trackers = Rc::new(RefCell::new(Trackers::new()));
    //     let mut state = EncounterState::new(trackers, options.version.clone());

    //     let entity = create_player_stats();
    //     state.encounter.entities.insert(entity.name.clone(), entity);
    //     state.last_update = Instant::now() + options.duration;

    //     let mut event_emitter = MockEventEmitter::new();

    //     event_emitter
    //         .expect_emit()
    //         .with(always(), always())
    //         .returning(|_, _: Option<Encounter>| anyhow::Ok(()));

    //     let event_emitter = Arc::new(event_emitter);

    //     let event_listener = create_and_setup_event_listener();
    //     let region_store = create_and_setup_region_store();
    //     let local_player_store = create_and_setup_local_player_store();

    //     let repository = MockEncounterService::new();
    //     let repository = Arc::new(repository);
        
    //     let heartbeat_api = MockHeartbeatApi::new();
    //     let heartbeat_api = Arc::new(Mutex::new(heartbeat_api));
        
    //     let stats_api = MockStatsApi::new();
    //     let stats_api = Arc::new(Mutex::new(stats_api));

    //     start(
    //         flags,
    //         packet_sniffer,
    //         &mut packet_handler,
    //         &mut state,
    //         options,
    //         event_emitter,
    //         event_listener, 
    //         region_store, 
    //         local_player_store,
    //         repository,
    //         heartbeat_api,
    //         stats_api
    //     ).unwrap();
    // }
}