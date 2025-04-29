
use crate::encounter_state::EncounterState;
use crate::models::CompleteEncounter;
use super::flags::Flags;
use super::interval_timer::IntervalTimer;
use super::packet_handler::PacketHandler;
use super::utils::{save_to_db, send_to_ui};
use super::{abstractions::*, register_listeners};
use anyhow::Result;
use chrono::{Duration, Utc};
use hashbrown::HashMap;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_store::encounter_service::EncounterService;
use tokio::sync::Mutex;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

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

pub async fn start<'a, FL, PS, PH, DH, EE, EL, RS, LP, PE, HB, SA>(
    flags: Arc<FL>,
    mut packet_sniffer: PS,
    packet_handler: &mut PH,
    damage_encryption_handler: Arc<DH>, 
    state: &mut EncounterState,
    options: StartOptions,
    event_emitter: Arc<EE>,
    event_listener: Arc<EL>,
    region_store: Arc<RS>,
    local_player_store: Arc<RwLock<LP>>,
    persister: Arc<PE>,
    heartbeat_api: &mut HB,
    stats_api: Arc<SA>) 
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
        PE: Persister,
        HB: HeartbeatApi,
        SA: StatsApi
    {
    let mut last_update = IntervalTimer::new(options.duration);
    let mut last_party_update = IntervalTimer::new(options.party_duration);
    
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

    loop {
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
            
            state.party_info = state.get_party();

            if let Some(encounter) = state.get_complete_encounter() {
                persister.save(&options.version, encounter)?;
            }

            state.saved = true;
            state.resetting = true;
        }

        if flags.triggered_boss_only_damage() {
            state.boss_only_damage = true;
        } else {
            state.boss_only_damage = false;
        }

        let now = Utc::now();

        tokio::select! {
            result = packet_sniffer.recv() => {
                match result {
                    Some((op, data)) => {
                        match packet_handler.handle(op, &data, state, &options) {
                            Err(_) => {
                
                            },
                            _ => {}
                        }
                    },
                    None => return Ok(()),
                }
            }
        }

        let can_send_to_ui = last_update.has_elapsed(now) || state.resetting || state.boss_dead_update;

        if can_send_to_ui {
            send_to_ui(now, state, event_emitter.clone(), &options);
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
            let version = options.version.clone();
            
            if let Some(region) = state.region.clone() {
                heartbeat_api.beat(client_id, version, region);
            }
        }

        #[cfg(test)]
        break;
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
    use crate::{flags::MockFlags, test_utils::*};
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

        let mut state = EncounterState::new();

        packet_capturer_builder.start(&mut state).await;
       
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

        let mut state = EncounterState::new();

        packet_capturer_builder.start(&mut state).await;
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

        let mut state = EncounterState::new();
        state.resetting = true;
        state.saved = true;
        state.party_freeze = true;
        state.party_cache = Some(vec![]);

        packet_capturer_builder.start(&mut state).await;

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
            .setup_persister()
            .setup_damage_encryption_handler();

        let mut state = EncounterState::new();
        update_state_with_player_and_boss(&mut state);

        packet_capturer_builder.start(&mut state).await;
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

            let mut state = EncounterState::new();
        let options = packet_capturer_builder.get_options();
        let entity = create_player_stats();
        state.entity_stats.insert(entity.id, entity);

        packet_capturer_builder.start(&mut state).await;
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

        let mut state = EncounterState::new();
        let options = packet_capturer_builder.get_options();
        let entity = create_player_stats();
        let entity = create_player_stats();
        state.entity_stats.insert(entity.id, entity);

        packet_capturer_builder.start(&mut state).await;
    }
}