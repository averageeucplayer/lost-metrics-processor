use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::{abstractions::*, flags::MockFlags, packet_handler::*};
use crate::test_utils::*;
use lost_metrics_core::models::LocalInfo;
use mockall::*;

pub struct PacketHandlerBuilder {
    damage_encryption_handler: MockDamageEncryptionHandlerTrait,
    event_emitter: MockEventEmitter,
    region_store: MockRegionStore,
    local_player_store: MockLocalPlayerStore,
    persister: MockPersister,
    flags: MockFlags,
}

impl PacketHandlerBuilder {
    pub fn new() -> Self {
        let damage_encryption_handler = MockDamageEncryptionHandlerTrait::new();
        let event_emitter = MockEventEmitter::new();
        let region_store = MockRegionStore::new();
        let local_player_store = MockLocalPlayerStore::new();
        let persister = MockPersister::new();
        let flags = MockFlags::new();

        Self {
            damage_encryption_handler,
            region_store,
            local_player_store,
            persister,
            event_emitter,
            flags
        }
    }

    pub fn setup_damage_encryption_handler(mut self) -> Self {
        self.damage_encryption_handler
            .expect_update_zone_instance_id()
            .returning(|_| {});

        self
    }

    pub fn ensure_region_getter_called(&mut self, region_name: String) {
        self.region_store
            .expect_get()
            .returning(move || Some(region_name.clone()));
    }

    pub fn ensure_save_to_db_called(&mut self) {
        self.persister
            .expect_save()
            .with(predicate::always(), predicate::always())
            .returning(|_,_| Ok(()));
    }

    pub fn ensure_event_called(&mut self) {
        self.event_emitter
            .expect_emit()
            .with(predicate::always())
            .returning(|_| Ok(()));
    }

    pub fn ensure_flag_can_emit_details_called(&mut self, value: bool) {
        self.flags
            .expect_can_emit_details()
            .with()
            .returning(move || value);
    }

    pub fn setup_local_store_get(&mut self, local_info: LocalInfo) {
        self.local_player_store
            .expect_get()
            .return_const(local_info);
    }

    
    pub fn ensure_event_decrypted(&mut self) {
        self.damage_encryption_handler
            .expect_decrypt_damage_event()
            .return_const(true);
    }

    pub fn ensure_local_store_write_called(&mut self) {
        self.local_player_store
            .expect_write()
            .returning(|_, _| Ok(()));
    }

    pub fn build(self) -> 
    DefaultPacketHandler<
        MockFlags,
        MockDamageEncryptionHandlerTrait,
        MockStatsApi,
        MockRegionStore,
        MockLocalPlayerStore,
        MockEventEmitter,
        MockPersister> {
        let event_emitter = Arc::new(self.event_emitter);
        let region_store = Arc::new(self.region_store);
        let local_player_store = Arc::new(RwLock::new(self.local_player_store));
        let persister = Arc::new(self.persister);
        let stats_api = Arc::new(MockStatsApi::new());
        let flags = Arc::new(self.flags);
        let damage_encryption_handler= Arc::new(self.damage_encryption_handler);

        let packet_handler = DefaultPacketHandler::new(
            flags.clone(),
            damage_encryption_handler,
            local_player_store,
            event_emitter,
            region_store,
            persister,
            stats_api
        );

        packet_handler
    }
}
