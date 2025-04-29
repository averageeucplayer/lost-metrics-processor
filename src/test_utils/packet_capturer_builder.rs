use std::sync::{Arc, RwLock};

use lost_metrics_core::models::LocalInfo;
use mockall::predicate::{self, always};

use crate::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::MockPacketHandler, start, StartOptions};

use super::{create_start_options, MockDamageEncryptionHandlerTrait};

pub struct PacketCapturerBuilder {
    options: StartOptions,
    flags: MockFlags,
    packet_sniffer: MockPacketSniffer,
    packet_handler: MockPacketHandler,
    event_emitter: MockEventEmitter,
    event_listener: MockEventListener,
    region_store: MockRegionStore,
    local_player_store: MockLocalPlayerStore,
    persister: MockPersister,
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
        let heartbeat_api = MockHeartbeatApi::new();        
        let stats_api = MockStatsApi::new();
        let persister = MockPersister::new();
        let damage_encryption_handler = MockDamageEncryptionHandlerTrait::new();

        Self {
            options,
            flags,
            packet_handler,
            packet_sniffer,
            stats_api,
            heartbeat_api,
            persister,
            event_emitter,
            event_listener,
            damage_encryption_handler,
            local_player_store,
            region_store,
        }
    }

    pub fn setup_persister(mut self) -> Self {
        self.persister
            .expect_save()
            .with(predicate::always(), predicate::always())
            .returning(|_,_| Ok(()));

        self
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
            .with(always())
            .returning(|_| anyhow::Ok(()));

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
            .expect_start()
            .returning(|_, _| Ok(()));

        self.packet_sniffer
            .expect_recv()
            .returning(|| Some((Pkt::InitEnv, vec![])));

        self
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

    pub async fn start(mut self, state: &mut EncounterState)  {
        let flags = Arc::new(self.flags);
        let local_player_store = Arc::new(RwLock::new(self.local_player_store));
        let event_emitter = Arc::new(self.event_emitter);
        let event_listener = Arc::new(self.event_listener);
        let region_store  = Arc::new(self.region_store);
        let damage_encryption_handler = Arc::new(self.damage_encryption_handler);
        let mut heartbeat_api = self.heartbeat_api;
        let stats_api = Arc::new(self.stats_api);
        let persister = Arc::new(self.persister);

        start(
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
            persister,
            &mut heartbeat_api,
            stats_api
        ).await;
    }
}