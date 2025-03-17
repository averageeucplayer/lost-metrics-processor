use lost_metrics_core::models::{Entity, EntityType};
use lost_metrics_misc::get_npc_entity_type_name_grade;
use lost_metrics_sniffer_stub::packets::definitions::PKTNewNpc;

use super::EntityTracker;

impl EntityTracker {
        
    pub fn new_npc(&mut self, pkt: PKTNewNpc, max_hp: i64) -> Entity {
        let (entity_type, name, grade) = get_npc_entity_type_name_grade(
            pkt.npc_struct.object_id,
            pkt.npc_struct.type_id,
            max_hp);

        let npc = Entity {
            id: pkt.npc_struct.object_id,
            entity_type,
            name,
            grade,
            npc_id: pkt.npc_struct.type_id,
            level: pkt.npc_struct.level,
            balance_level: pkt.npc_struct.balance_level.unwrap_or(pkt.npc_struct.level),
            push_immune: entity_type == EntityType::Boss,
            stats: pkt
                .npc_struct
                .stat_pairs
                .iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };
        self.entities.insert(npc.id, npc.clone());
        self.status_tracker.borrow_mut().remove_local_object(npc.id);
        self.build_and_register_status_effects(pkt.npc_struct.status_effect_datas, npc.id);
        npc
    }
}