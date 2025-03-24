use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::*, stats_api::MockStatsApi, trackers::Trackers};
use crate::live::test_utils::*;
use lost_metrics_data::NPC_DATA;
use lost_metrics_sniffer_stub::packets::{definitions::{PKTNewPC, PKTNewPCInner}, structures::{NpcStruct, StatusEffectData}};
use lost_metrics_store::encounter_service;
use serde::Serialize;
use std::fmt::Debug;
use mockall::*;

pub fn get_npc_by_name<'a>(npc_name: &str) -> Option<&'a Npc> {
    NPC_DATA
        .iter()
        .filter(|(id, npc)| 
            npc.hp_bars > 1
            && npc.name.as_ref().filter(|name| *name == npc_name).is_some())
        .max_by_key(|(_, npc)| npc.grade)
        .map(|(_, npc)| npc)
}

pub fn create_npc(object_id: u64, name: &str) -> PKTNewNpc {

    let npc = get_npc_by_name(name).expect("Provide valid npc name");

    PKTNewNpc {
        npc_struct: NpcStruct {
            type_id: npc.id as u32,
            object_id,
            level: 60,
            balance_level: None,
            stat_pairs: vec![],
            status_effect_datas: vec![],
        }   
    }
}

pub fn create_pc(player_id: u64, class_id: u32, character_id: u64, name: String) -> PKTNewPC {
    PKTNewPC { 
        pc_struct: PKTNewPCInner { 
            player_id,
            name,
            class_id,
            max_item_level: 1.0,
            character_id,
            stat_pairs: vec![],
            equip_item_datas: vec![],
            status_effect_datas: vec![]
        }
    }
}

pub struct PacketHandlerBuilder {
    trackers: Rc<RefCell<Trackers>>,
    state: EncounterState,
    damage_encryption_handler: MockDamageEncryptionHandlerTrait,
    event_emitter: MockEventEmitter,
    region_store: MockRegionStore,
    local_player_store: MockLocalPlayerStore,
    encounter_service: MockEncounterService,
    flags: MockFlags,
}

impl PacketHandlerBuilder {
    pub fn new() -> Self {
        let trackers = Trackers::new();
        let damage_encryption_handler = MockDamageEncryptionHandlerTrait::new();
        let trackers = Rc::new(RefCell::new(trackers));
        let state = EncounterState::new(trackers.clone(), "0.0.1".into());
        let event_emitter = MockEventEmitter::new();
        let region_store = MockRegionStore::new();
        let local_player_store = MockLocalPlayerStore::new();
        let encounter_service = MockEncounterService::new();
        let stats_api = Arc::new(Mutex::new(MockStatsApi::new()));
        let flags = MockFlags::new();

        Self {
            trackers,
            damage_encryption_handler,
            state,
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

    pub fn ensure_local_store_write_called(&mut self) {
        self.local_player_store
            .expect_write()
            .returning(|_, _| Ok(()));
    }

    pub fn set_local_player_id(&mut self, id: u64) {
        self.trackers.borrow_mut().entity_tracker.local_entity_id = id;
    }

    pub fn add_party_status_effect(&mut self, character_id: u64, instance_id: u32, buff_id: u32) {
        let entity_tracker = &mut self.trackers.borrow_mut().entity_tracker;
        let packet = PKTPartyStatusEffectAddNotify {
            character_id,
            status_effect_datas: vec![StatusEffectData {
                source_id: 1,
                status_effect_id: buff_id,
                status_effect_instance_id: instance_id,
                value: Some(vec![]),
                total_time: 10.0,
                stack_count: 1,
                end_tick: 1
            }]
        };
        entity_tracker.party_status_effect_add(packet, &self.state.encounter.entities);
    }

    pub fn create_player_with_character_id(&mut self, player_id: u64, character_id: u64, name: String) {
        let playable_character = create_pc(player_id, 101, character_id, name);
        let entity = self.trackers.borrow_mut().entity_tracker.new_pc(playable_character);
        self.state.on_new_pc(entity, 100000, 100000);
    }

    pub fn create_player(&mut self, player_id: u64, name: String) {
        let playable_character = create_pc(player_id, 101, 1, name);
        let entity = self.trackers.borrow_mut().entity_tracker.new_pc(playable_character);
        self.state.on_new_pc(entity, 100000, 100000);
    }

    pub fn create_npc(&mut self, object_id: u64, name: &str) {
        let npc = create_npc(object_id, name);
        let entity = self.trackers.borrow_mut().entity_tracker.new_npc(npc, 100000);
        self.state.on_new_npc(entity, 100000, 100000);
    }

    pub fn build(self) -> 
    (EncounterState, DefaultPacketHandler<
        MockFlags,
        MockDamageEncryptionHandlerTrait,
        MockStatsApi,
        MockRegionStore,
        MockLocalPlayerStore,
        MockEventEmitter,
        MockEncounterService>) {
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
            self.trackers.clone(),
            local_player_store,
            event_emitter,
            region_store,
            repository,
            stats_api
        );

        (self.state, packet_handler)
    }
}
