
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

        let can_send_to_ui = state.last_update.elapsed() >= options.duration || state.resetting || state.boss_dead_update;

        if can_send_to_ui {
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
    use tokio::time::sleep;
    use crate::live::{flags::MockFlags, heartbeat_api::MockHeartbeatApi, stats_api::MockStatsApi, test_utils::*, trackers::Trackers};
    use crate::live::test_utils::MockEncounterService;
    use super::*;

    #[tokio::test]
    async fn should_handle_packet() {

        let mut packet_capturer_builder = PacketCapturerBuilder::new();

        let mut packet_capturer_builder = packet_capturer_builder
            .setup_default_flags()
            .setup_event_listener()
            .setup_local_player_store()
            .setup_region_store()
            .setup_packet_sniffer()
            .setup_packet_handler()
            .setup_damage_encryption_handler();

        let mut state = packet_capturer_builder.get_state();

        packet_capturer_builder.start(&mut state);
       
    }

    #[tokio::test]
    async fn should_reset_requested_from_ui() {

        let mut packet_capturer_builder = PacketCapturerBuilder::new();

        let mut packet_capturer_builder = packet_capturer_builder
            .setup_flags(false, true, false, false, false)
            .setup_event_listener()
            .setup_local_player_store()
            .setup_region_store()
            .setup_packet_sniffer()
            .setup_packet_handler()
            .setup_damage_encryption_handler();

        let mut state = packet_capturer_builder.get_state();

        packet_capturer_builder.start(&mut state);
    }

    #[tokio::test]
    async fn should_reset_on_flag() {
        let mut packet_capturer_builder = PacketCapturerBuilder::new();

        let mut packet_capturer_builder = packet_capturer_builder
            .setup_default_flags()
            .setup_event_listener()
            .setup_local_player_store()
            .setup_region_store()
            .setup_packet_sniffer()
            .setup_packet_handler()
            .setup_damage_encryption_handler();

        let mut state = packet_capturer_builder.get_state();
        state.resetting = true;
        state.saved = true;
        state.party_freeze = true;
        state.party_cache = Some(vec![]);

        packet_capturer_builder.start(&mut state);

        assert_eq!(state.resetting, false);
        assert_eq!(state.saved, false);
        assert_eq!(state.party_freeze, false);
        assert_eq!(state.party_cache, None);
        assert!(state.party_map_cache.is_empty());
    }

    #[tokio::test]
    async fn should_save_to_db() {

        let mut packet_capturer_builder = PacketCapturerBuilder::new();

        let mut packet_capturer_builder = packet_capturer_builder
            .setup_flags(false, false, false, true, false)
            .setup_event_listener()
            .setup_local_player_store()
            .setup_region_store()
            .setup_packet_sniffer()
            .setup_packet_handler()
            .setup_damage_encryption_handler();

        let mut state = packet_capturer_builder.get_state();
        update_state_with_player_and_boss(&mut state);

        packet_capturer_builder.start(&mut state);
    }

    #[tokio::test]
    async fn should_send_party_info_to_ui() {

        let mut packet_capturer_builder = PacketCapturerBuilder::new();

        let mut packet_capturer_builder = packet_capturer_builder
            .setup_default_flags()
            .setup_event_listener()
            .setup_event_emitter()
            .setup_local_player_store()
            .setup_region_store()
            .setup_packet_sniffer()
            .setup_packet_handler()
            .setup_damage_encryption_handler();

        let mut state = packet_capturer_builder.get_state();
        let options = packet_capturer_builder.get_options();
        let entity = create_player_stats();
        state.encounter.entities.insert(entity.name.clone(), entity);
        state.last_update = Instant::now() + options.duration;
        state.last_party_update = Instant::now() + options.party_duration;

        sleep(Duration::from_secs(1)).await;
        packet_capturer_builder.start(&mut state);
    }

    #[tokio::test]
    async fn should_send_encounter_to_ui() {

        let mut packet_capturer_builder = PacketCapturerBuilder::new();

        let mut packet_capturer_builder = packet_capturer_builder
            .setup_default_flags()
            .setup_event_listener()
            .setup_event_emitter()
            .setup_local_player_store()
            .setup_region_store()
            .setup_packet_sniffer()
            .setup_packet_handler()
            .setup_damage_encryption_handler();

        let mut state = packet_capturer_builder.get_state();
        let options = packet_capturer_builder.get_options();
        let entity = create_player_stats();
        let entity = create_player_stats();
        state.encounter.entities.insert(entity.name.clone(), entity);
        state.last_update = Instant::now() + options.duration;

        packet_capturer_builder.start(&mut state);
    }
}