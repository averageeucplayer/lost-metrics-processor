use lost_metrics_core::models::{Entity, EntityType};
use lost_metrics_sniffer_stub::packets::definitions::PKTNewPC;

use super::{truncate_gear_level, EntityTracker};

impl EntityTracker {

    pub fn new_pc(&mut self, pkt: PKTNewPC) -> Entity {
        let entity = Entity {
            id: pkt.pc_struct.player_id,
            entity_type: EntityType::Player,
            name: pkt.pc_struct.name.clone(),
            class_id: pkt.pc_struct.class_id as u32,
            gear_level: truncate_gear_level(pkt.pc_struct.max_item_level), // todo?
            character_id: pkt.pc_struct.character_id,
            stats: pkt
                .pc_struct
                .stat_pairs
                .iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };

        self.entities.insert(entity.id, entity.clone());
        let old_entity_id = self
            .id_tracker
            .borrow()
            .get_entity_id(pkt.pc_struct.character_id);
        if let Some(old_entity_id) = old_entity_id {
            self.party_tracker
                .borrow_mut()
                .change_entity_id(old_entity_id, entity.id);
        }
        self.id_tracker
            .borrow_mut()
            .add_mapping(pkt.pc_struct.character_id, pkt.pc_struct.player_id);
        self.party_tracker
            .borrow_mut()
            .complete_entry(pkt.pc_struct.character_id, pkt.pc_struct.player_id);
        // println!("party status: {:?}", self.party_tracker.borrow().character_id_to_party_id);
        let local_character_id = if self.local_character_id != 0 {
            self.local_character_id
        } else {
            self.id_tracker
                .borrow()
                .get_local_character_id(self.local_entity_id)
        };
        self.status_tracker
            .borrow_mut()
            .new_pc(pkt, local_character_id);
        entity
    }
}