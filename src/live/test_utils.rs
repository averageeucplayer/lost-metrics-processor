use std::cell::RefCell;
use std::rc::Rc;
use std::sync::RwLock;
use std::sync::Arc;
use chrono::Duration;
use chrono::Utc;
use lost_metrics_core::models::DamageStats;
use lost_metrics_core::models::Encounter;
use lost_metrics_core::models::EncounterEntity;
use lost_metrics_core::models::EntityType;
use lost_metrics_core::models::LocalInfo;
use mockall::predicate::always;
use tokio::sync::Mutex;
use super::encounter_state::EncounterState;
use super::flags::MockFlags;
use super::heartbeat_api::MockHeartbeatApi;
use super::stats_api::MockStatsApi;
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

pub fn create_start_options() -> StartOptions {
    StartOptions {
        version: "0.0.1".into(),
        port: 420,
        database_path: "encounter.db".into(),
        local_player_path: "local_players.json".into(),
        raid_end_capture_timeout: Duration::seconds(10),
        region_path: "current_region".into(),
        duration: Duration::milliseconds(500),
        party_duration: Duration::milliseconds(200),
    }
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

pub struct PacketCapturerBuilder {
    options: StartOptions,
    flags: MockFlags,
    packet_sniffer: MockPacketSniffer,
    packet_handler: MockPacketHandler,
    event_emitter: MockEventEmitter,
    event_listener: MockEventListener,
    region_store: MockRegionStore,
    local_player_store: MockLocalPlayerStore,
    encounter_service: MockEncounterService,
    heartbeat_api: MockHeartbeatApi,
    stats_api: MockStatsApi,
    damage_encryption_handler: MockDamageEncryptionHandlerTrait
}

impl PacketCapturerBuilder {
    pub fn new() -> Self {

        let options = create_start_options();
        let flags = MockFlags::new();
        let packet_sniffer  = MockPacketSniffer::new();
        let mut packet_handler  = MockPacketHandler::new();
        let mut state = EncounterState::new();
        let event_emitter = MockEventEmitter::new();
        let event_listener = MockEventListener::new();
        let region_store = MockRegionStore::new();
        let local_player_store = MockLocalPlayerStore::new();
        let encounter_service = MockEncounterService::new();
        let heartbeat_api = MockHeartbeatApi::new();        
        let stats_api = MockStatsApi::new();
        let damage_encryption_handler = MockDamageEncryptionHandlerTrait::new();

        Self {
            options,
            flags,
            packet_handler,
            packet_sniffer,
            stats_api,
            heartbeat_api,
            encounter_service,
            event_emitter,
            event_listener,
            damage_encryption_handler,
            local_player_store,
            region_store,
        }
    }

    pub fn setup_flags(mut self, stop: bool, reset: bool, pause: bool, save: bool, boss_only_damage: bool) -> Self {
        self.flags
            .expect_triggered_stop()
            .returning(move || stop);

        self.flags
            .expect_triggered_reset()
            .returning(move || reset);

        self.flags
            .expect_clear_reset()
            .returning(|| {});

        self.flags
            .expect_triggered_pause()
            .returning(move || pause);

        self.flags
            .expect_triggered_save()
            .returning(move || save);

        self.flags
            .expect_reset_save()
            .return_once(|| {});

        self.flags
            .expect_triggered_boss_only_damage()
            .returning(move || boss_only_damage);

        self
    }

    pub fn setup_default_flags(mut self) -> Self {
        self.setup_flags(false, false, false, false, false)
    }

    pub fn setup_event_emitter(mut self) -> Self {
        
        self.event_emitter
            .expect_emit()
            .with(always(), always())
            .returning(|_, _: Option<Encounter>| anyhow::Ok(()));

        self
    }

    pub fn setup_event_listener(mut self) -> Self {
    
        self.event_listener
            .expect_listen_global()
            .times(5)
            .returning(|_, _| {});
    
        self
    }
    
    pub fn setup_packet_handler(mut self) -> Self {

        self.packet_handler
            .expect_handle()
            .returning(|_, _, _, _| Ok(()));

        self
    }
    
    pub fn setup_packet_sniffer(mut self) -> Self {
        use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    
        self.packet_sniffer
            .expect_start_capture()
            .returning(Self::setup_receiver);

        self
    }

    fn setup_receiver(_port: u16, _str: String) -> anyhow::Result<Box<dyn ReceiverWrapper>> {
        use lost_metrics_sniffer_stub::packets::opcodes::Pkt;

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
    }

    pub fn setup_region_store(mut self) -> Self {
    
        self.region_store
            .expect_get_path()
            .returning(|| "path".into());
    
        self.region_store
            .expect_get()
            .returning(|| None);

        self
    }
    
    pub fn setup_local_player_store(mut self) -> Self {
        self.local_player_store
            .expect_load()
            .returning(|| Ok(false));
    
        let local_info = LocalInfo::default();
    
        self.local_player_store
            .expect_get()
            .return_const(local_info);
        
        self
    }

    pub fn setup_damage_encryption_handler(mut self) -> Self {
        self.damage_encryption_handler
            .expect_start()
            .returning(|| Ok(()));
    
        self
    }

    pub fn get_options(&mut self) -> &mut StartOptions {
        &mut self.options
    }

    pub fn start(mut self, state: &mut EncounterState)  {
        let flags = Arc::new(self.flags);
        let local_player_store = Arc::new(RwLock::new(self.local_player_store));
        let event_emitter = Arc::new(self.event_emitter);
        let encounter_service = Arc::new(self.encounter_service);
        let event_listener = Arc::new(self.event_listener);
        let region_store  = Arc::new(self.region_store);
        let damage_encryption_handler = Arc::new(self.damage_encryption_handler);
        let heartbeat_api = Arc::new(Mutex::new(self.heartbeat_api));
        let stats_api = Arc::new(Mutex::new(self.stats_api));

        super::start(
            flags,
            self.packet_sniffer,
            &mut self.packet_handler,
            damage_encryption_handler,
            state,
            self.options,
            event_emitter,
            event_listener, 
            region_store, 
            local_player_store,
            encounter_service,
            heartbeat_api,
            stats_api
        ).unwrap();
    }
}