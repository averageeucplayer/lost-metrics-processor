use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::*, stats_api::MockStatsApi, trackers::Trackers};
use crate::live::test_utils::MockEncounterService;
use lost_metrics_sniffer_stub::packets::definitions::{PKTNewPC, PKTNewPCInner};
use serde::Serialize;
use std::fmt::Debug;
use mockall::*;

pub fn create_random_pc(player_id: u64, name: String) -> PKTNewPC {
    PKTNewPC { 
        pc_struct: PKTNewPCInner { 
            player_id,
            name,
            class_id: 101,
            max_item_level: 1.0,
            character_id: 1,
            stat_pairs: vec![],
            equip_item_datas: vec![],
            status_effect_datas: vec![]
        }
    }
}

pub struct PacketHandlerBuilder {
    trackers: Rc<RefCell<Trackers>>,
    state: EncounterState,
    event_emitter: MockEventEmitter,
    region_store: MockRegionStore,
    flags: MockFlags,
}

impl PacketHandlerBuilder {
    pub fn new() -> Self {
        let trackers = Trackers::new();
        let trackers = Rc::new(RefCell::new(trackers));
        let state = EncounterState::new(trackers.clone(), "0.0.1".into());
        let event_emitter = MockEventEmitter::new();
        let region_store = MockRegionStore::new();
        let flags = MockFlags::new();

        Self {
            trackers,
            state,
            region_store,
            event_emitter,
            flags
        }
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

    pub fn create_player(&mut self, player_id: u64, name: String) {
        let playable_character = create_random_pc(player_id, name);
        let entity = self.trackers.borrow_mut().entity_tracker.new_pc(playable_character);
        self.state.on_new_pc(entity, 100000, 100000);
    }

    pub fn build(self) -> 
    (EncounterState, DefaultPacketHandler<
        MockFlags,
        MockStatsApi,
        MockRegionStore,
        MockLocalPlayerStore,
        MockEventEmitter,
        MockEncounterService>) {
        let event_emitter = Arc::new(self.event_emitter);
        let region_store = Arc::new(self.region_store);
        let local_player_store = Arc::new(RwLock::new(MockLocalPlayerStore::new()));
        let repository = Arc::new(MockEncounterService::new());
        let stats_api = Arc::new(Mutex::new(MockStatsApi::new()));
        let flags = Arc::new(self.flags);

        let packet_handler = DefaultPacketHandler::new(
            flags.clone(),
            self.trackers.clone(),
            local_player_store.clone(),
            event_emitter.clone(),
            region_store.clone(),
            repository.clone(),
            stats_api.clone(),
        );

        (self.state, packet_handler)
    }
}
