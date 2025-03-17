use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore, Repository};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use lost_metrics_sniffer_stub::packets::definitions::*;

use super::DefaultPacketHandler;

impl<FL, SA, RS, LP, EE, RE> DefaultPacketHandler<FL, SA, RS, LP, EE, RE>
where
    FL: Flags,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    RE: Repository {
    pub fn on_counterattack(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {
        let packet = parse_pkt1(&data, PKTCounterAttackNotify::new)?;
        let source_id = packet.source_id;
        
        if let Some(entity) = self.trackers.borrow().entity_tracker.entities.get(&source_id) {
            state.on_counterattack(entity);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
    use lost_metrics_sniffer_stub::packets::{definitions::PKTCounterAttackNotify, opcodes::Pkt};
    use tokio::{runtime::Handle, sync::Mutex};
    use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::{test_utils::create_random_pc, DefaultPacketHandler, PacketHandler}, stats_api::MockStatsApi, test_utils::create_start_options, trackers::Trackers};

    #[tokio::test]
    async fn should_update_stats_when_counter() {
        let options = create_start_options();
        let event_emitter = Arc::new(MockEventEmitter::new());
        let region_store = Arc::new(MockRegionStore::new());
        let local_player_store = Arc::new(RwLock::new(MockLocalPlayerStore::new()));
        let repository = Arc::new(MockRepository::new());
        let stats_api = Arc::new(Mutex::new(MockStatsApi::new()));
        let flags = Arc::new(MockFlags::new());
        let mut trackers = Trackers::new();
    
        let playable_character = create_random_pc(1, "test".into());
        let entity = trackers.entity_tracker.new_pc(playable_character);
        let entity_name = entity.name.clone();
        
        let opcode = Pkt::CounterAttackNotify;
        let data = PKTCounterAttackNotify {
            source_id: 1
        };
        let data = data.encode().unwrap();
    
        let trackers = Rc::new(RefCell::new(trackers));
        let mut state = EncounterState::new(trackers.clone(), options.version.clone());
    
        state.on_new_pc(entity, 100000, 100000);
    
        let mut packet_handler = DefaultPacketHandler::new(
            flags.clone(),
            trackers.clone(),
            local_player_store.clone(),
            event_emitter.clone(),
            region_store.clone(),
            repository.clone(),
            stats_api.clone(),
        );
    
        let rt = Handle::current();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    
        assert_eq!(state.encounter.entities.get(&entity_name).unwrap().skill_stats.counters, 1);
    }
}