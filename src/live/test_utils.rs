use std::sync::RwLock;
use std::time::Duration;
use std::sync::Arc;
use chrono::Utc;
use lost_metrics_core::models::DamageStats;
use lost_metrics_core::models::EncounterEntity;
use lost_metrics_core::models::EntityType;
use lost_metrics_core::models::LocalInfo;
use super::encounter_state::EncounterState;
use super::flags::MockFlags;
use super::StartOptions;
use super::abstractions::*;
use super::packet_handler::MockPacketHandler;
use lost_metrics_store::encounter_service::EncounterService;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::structures::SkillDamageEvent;

#[cfg(test)]
use mockall::mock;

#[cfg(test)]
use lost_metrics_store::models::CreateEncounter;

#[cfg(test)]
mock! {
    pub DamageEncryptionHandlerTrait {}
    impl DamageEncryptionHandlerTrait for DamageEncryptionHandlerTrait {
        fn start(&self) -> anyhow::Result<()>;
        fn decrypt_damage_event(&self, event: &mut SkillDamageEvent) -> bool;
        fn update_zone_instance_id(&self, channel_id: u32);
    }
}


#[cfg(test)]
mock! {
    pub EncounterService {}
    impl EncounterService for EncounterService {
        fn create(&self, payload: CreateEncounter) -> anyhow::Result<i64>;
    }
}

pub fn create_and_setup_flags() -> Arc<MockFlags> {
    let mut flags = MockFlags::new();

    flags
        .expect_triggered_stop()
        .returning(|| false);

    flags
        .expect_triggered_reset()
        .returning(|| false);

    flags
        .expect_triggered_pause()
        .returning(|| false);

    flags
        .expect_triggered_save()
        .returning(|| false);

    flags
        .expect_triggered_boss_only_damage()
        .returning(|| false);

    let flags = Arc::new(flags);

    flags
}

pub fn create_and_setup_event_listener() -> Arc<MockEventListener> {
    let mut event_listener = MockEventListener::new();

    event_listener
        .expect_listen_global()
        .times(5)
        .returning(|_, _| {});

    let event_listener = Arc::new(event_listener);

    event_listener
}

pub fn create_and_setup_packet_handler() -> MockPacketHandler {
    let mut packet_handler = MockPacketHandler::new();

    packet_handler
        .expect_handle()
        .returning(|_, _, _, _, _| Ok(()));

    packet_handler
}

pub fn create_and_setup_packet_sniffer() -> MockPacketSniffer {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;

    let mut packet_sniffer = MockPacketSniffer::new();

    packet_sniffer
        .expect_start_capture()
        .returning(move |_, _| {
            let mut receiver = MockReceiverWrapper::new();

            receiver
                .expect_recv()
                .times(1)
                .returning(|| {
                    let data = vec![];
                    Ok((Pkt::NewPC, data))
                });

            receiver
                .expect_recv()
                .times(1)
                .returning(|| Err(anyhow::anyhow!("End")));

            Ok(Box::new(receiver))
        });

    packet_sniffer
}

pub fn create_start_options() -> StartOptions {
    StartOptions {
        version: "0.0.1".into(),
        port: 420,
        database_path: "encounter.db".into(),
        local_player_path: "local_players.json".into(),
        raid_end_capture_timeout: Duration::from_secs(10),
        region_path: "current_region".into(),
        duration: Duration::from_millis(500),
        party_duration: Duration::from_millis(200),
    }
}

pub fn create_and_setup_region_store() -> Arc<MockRegionStore> {
    let mut region_store = MockRegionStore::new();

    region_store
        .expect_get_path()
        .returning(|| "path".into());

    region_store
        .expect_get()
        .returning(|| None);

    let region_store  = Arc::new(region_store);

    region_store
}

pub fn create_and_setup_local_player_store() -> Arc<RwLock<MockLocalPlayerStore>> {
    let mut local_player_store = MockLocalPlayerStore::new();

    local_player_store
        .expect_load()
        .returning(|| Ok(false));

    let local_info = LocalInfo::default();

    local_player_store
        .expect_get()
        .return_const(local_info);
    
    let local_player_store = Arc::new(RwLock::new(local_player_store));

    local_player_store
}

pub fn create_player_stats() -> EncounterEntity {
    use lost_metrics_core::models::{DamageStats, EntityType};

    let entity = EncounterEntity {
        id: 1,
        character_id: 1,
        name: "test".into(),
        entity_type: EntityType::Player,
        class_id: 101,
        damage_stats: DamageStats {
            damage_dealt: 1,
            ..Default::default()    
        },
        ..Default::default()
    };

    entity
}

pub fn update_state_with_player_and_boss(state: &mut EncounterState) {
    state.encounter.fight_start = Utc::now().timestamp_millis();
    
    let player = EncounterEntity {
        id: 1,
        entity_type: EntityType::Player,
        name: "test_player".into(),
        damage_stats: DamageStats {
            damage_dealt: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    
    state.encounter.entities.insert(player.name.clone(), player);

    let boss = EncounterEntity {
        id: 2,
        entity_type: EntityType::Boss,
        name: "test_boss".into(),
        current_hp: 0,
        max_hp: 1e9 as i64,
        ..Default::default()
    };
    state.encounter.current_boss_name = "test_boss".into();
    state.encounter.entities.insert(boss.name.clone(), boss);
}