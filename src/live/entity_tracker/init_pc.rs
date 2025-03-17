use lost_metrics_core::models::{Entity, EntityType};
use lost_metrics_sniffer_stub::packets::definitions::PKTInitPC;

use super::{truncate_gear_level, EntityTracker};

impl EntityTracker {

    pub fn init_pc(&mut self, pkt: PKTInitPC) -> Entity {
        let player = Entity {
            id: pkt.player_id,
            entity_type: EntityType::Player,
            name: pkt.name,
            class_id: pkt.class_id as u32,
            gear_level: truncate_gear_level(pkt.gear_level),
            character_id: pkt.character_id,
            stats: pkt
                .stat_pairs
                .iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };

        self.local_entity_id = player.id;
        self.local_character_id = player.character_id;
        self.entities.clear();
        self.entities.insert(player.id, player.clone());
        self.id_tracker
            .borrow_mut()
            .add_mapping(player.character_id, player.id);
        self.party_tracker
            .borrow_mut()
            .set_name(player.name.clone());
        self.party_tracker
            .borrow_mut()
            .complete_entry(player.character_id, player.id);
        self.status_tracker
            .borrow_mut()
            .remove_local_object(player.id);
        self.build_and_register_status_effects(pkt.status_effect_datas, player.id);
        player
    }
}