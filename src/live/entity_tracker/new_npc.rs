use lost_metrics_core::models::{Entity, EntityType};
use lost_metrics_misc::get_npc_entity_type_name_grade;
use lost_metrics_sniffer_stub::packets::definitions::PKTNewNpc;

use super::EntityTracker;

impl EntityTracker {
        
    pub fn new_npc(&mut self, pkt: PKTNewNpc, max_hp: i64) -> Entity {
        let type_id = pkt.npc_struct.type_id;
        let object_id = pkt.npc_struct.object_id;
        let status_effect_datas = pkt.npc_struct.status_effect_datas;

        let (entity_type, name, grade) = get_npc_entity_type_name_grade(
            object_id,
            type_id,
            max_hp);

        let npc = Entity {
            id: object_id,
            entity_type,
            name,
            grade,
            npc_id: type_id,
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
        self.entities.insert(object_id, npc.clone());
        self.status_tracker.borrow_mut().remove_local_object(object_id);
        self.build_and_register_status_effects(status_effect_datas, object_id);
        npc
    }
}