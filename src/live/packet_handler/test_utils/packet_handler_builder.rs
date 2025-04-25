use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::*, stats_api::MockStatsApi};
use crate::live::test_utils::*;
use lost_metrics_data::{NPC_DATA, SKILL_BUFF_DATA};
use lost_metrics_sniffer_stub::packets::{common::SkillMoveOptionData, definitions::{PKTNewPC, PKTNewPCInner}, structures::{NpcStruct, SkillDamageEvent, StatPair, StatusEffectData}};
use lost_metrics_store::encounter_service;
use serde::Serialize;
use std::fmt::Debug;
use mockall::*;

pub struct PacketHandlerBuilder {
    damage_encryption_handler: MockDamageEncryptionHandlerTrait,
    event_emitter: MockEventEmitter,
    region_store: MockRegionStore,
    local_player_store: MockLocalPlayerStore,
    encounter_service: MockEncounterService,
    flags: MockFlags,
}

impl PacketHandlerBuilder {
    pub fn new() -> Self {
        let damage_encryption_handler = MockDamageEncryptionHandlerTrait::new();
        let event_emitter = MockEventEmitter::new();
        let region_store = MockRegionStore::new();
        let local_player_store = MockLocalPlayerStore::new();
        let encounter_service = MockEncounterService::new();
        let stats_api = Arc::new(Mutex::new(MockStatsApi::new()));
        let flags = MockFlags::new();

        Self {
            damage_encryption_handler,
            region_store,
            local_player_store,
            encounter_service,
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

    pub fn ensure_event_called<S: Debug + Serialize + Clone + 'static>(&mut self, event_name: String) {
        self.event_emitter
            .expect_emit()
            .with(predicate::eq(event_name), predicate::always())
            .returning(|_, _: S| Ok(()));
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

    // pub fn set_local_player_id(&mut self, id: u64) {
    //     self.state.local_entity_id = id;
    // }

    // pub fn add_party_status_effect(&mut self, character_id: u64, instance_id: u32, buff_id: u32) {
    //     let packet = PKTPartyStatusEffectAddNotify {
    //         character_id,
    //         status_effect_datas: vec![StatusEffectData {
    //             source_id: 1,
    //             status_effect_id: buff_id,
    //             status_effect_instance_id: instance_id,
    //             value: Some(vec![]),
    //             total_time: 10.0,
    //             stack_count: 1,
    //             end_tick: 1
    //         }]
    //     };
    //     state.party_status_effect_add(packet, &self.state.encounter.entities);
    // }

    // pub fn create_player_with_character_id(&mut self, player_id: u64, character_id: u64, name: String) {
    //     let playable_character = create_pc(player_id, 101, character_id, name);
    //     let entity = state.new_pc(playable_character);
    //     self.state.on_new_pc(entity, 100000, 100000);
    // }

    // pub fn create_unknown(&mut self, object_id: u64) {
    //     let entity = Entity {
    //         id: object_id,
    //         entity_type: EntityType::Unknown,
    //         name: format!("{:x}", object_id),
    //         ..Default::default()
    //     };
    //     state.entities.insert(entity.id, entity.clone());
    // }

    // pub fn create_player(&mut self, player_id: u64, name: String) {
    //     let playable_character = create_pc(player_id, 101, 1, name);
    //     let entity = state.new_pc(playable_character);
    //     self.state.on_new_pc(entity, 100000, 100000);
    // }

    // pub fn create_npc_with_hp(&mut self, object_id: u64, name: &str, max_hp: i64) {
    //     let npc = create_npc(object_id, name);
    //     let entity = state.new_npc(npc, max_hp);
    //     self.state.on_new_npc(entity, max_hp, max_hp);
    // }

    // pub fn create_npc(&mut self, object_id: u64, name: &str) {
    //     let npc = create_npc(object_id, name);
    //     let entity = state.new_npc(npc, 100000);
    //     self.state.on_new_npc(entity, 100000, 100000);
    // }

    pub fn build(self) -> 
    DefaultPacketHandler<
        MockFlags,
        MockDamageEncryptionHandlerTrait,
        MockStatsApi,
        MockRegionStore,
        MockLocalPlayerStore,
        MockEventEmitter,
        MockEncounterService> {
        let event_emitter = Arc::new(self.event_emitter);
        let region_store = Arc::new(self.region_store);
        let local_player_store = Arc::new(RwLock::new(self.local_player_store));
        let repository = Arc::new(self.encounter_service);
        let stats_api = Arc::new(Mutex::new(MockStatsApi::new()));
        let flags = Arc::new(self.flags);
        let damage_encryption_handler= Arc::new(self.damage_encryption_handler);

        let packet_handler = DefaultPacketHandler::new(
            flags.clone(),
            damage_encryption_handler,
            local_player_store,
            event_emitter,
            region_store,
            repository,
            stats_api
        );

        packet_handler
    }
}
