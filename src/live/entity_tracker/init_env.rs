use log::info;
use lost_metrics_core::models::{Entity, EntityType};
use lost_metrics_sniffer_stub::packets::definitions::PKTInitEnv;

use super::EntityTracker;

impl EntityTracker {
    pub fn init_env(&mut self, pkt: PKTInitEnv) -> Entity {
        if !self.local_entity_id == 0 {
            let party_id = self
                .party_tracker
                .borrow_mut()
                .entity_id_to_party_id
                .get(&self.local_entity_id)
                .cloned();
            if let Some(party_id) = party_id {
                self.party_tracker
                    .borrow_mut()
                    .entity_id_to_party_id
                    .remove(&self.local_entity_id);
                self.party_tracker
                    .borrow_mut()
                    .entity_id_to_party_id
                    .insert(pkt.player_id, party_id);
            }
        }
    
        let mut local_player = self
            .entities
            .get(&self.local_entity_id)
            .cloned()
            .unwrap_or_else(|| Entity {
                entity_type: EntityType::Player,
                name: "You".to_string(),
                class_id: 0,
                gear_level: 0.0,
                character_id: self.local_character_id,
                ..Default::default()
            });
    
        info!("init env: eid: {}->{}", self.local_entity_id, pkt.player_id);
    
        local_player.id = pkt.player_id;
        self.local_entity_id = pkt.player_id;
    
        self.entities.clear();
        self.entities.insert(local_player.id, local_player.clone());
        self.id_tracker.borrow_mut().clear();
        self.status_tracker.borrow_mut().clear();
        if local_player.character_id > 0 {
            self.id_tracker
                .borrow_mut()
                .add_mapping(local_player.character_id, local_player.id);
            self.party_tracker
                .borrow_mut()
                .complete_entry(local_player.character_id, local_player.id);
        }
        local_player
    }
}