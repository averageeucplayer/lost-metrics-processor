use chrono::Utc;
use hashbrown::HashMap;
use lost_metrics_core::models::{EncounterEntity, StatusEffectDetails, StatusEffectTargetType, StatusEffectType};
use lost_metrics_sniffer_stub::packets::definitions::PKTPartyStatusEffectAddNotify;

use crate::live::status_tracker::build_status_effect;

use super::EntityTracker;

impl EntityTracker {

    pub fn party_status_effect_add(
        &mut self,
        pkt: PKTPartyStatusEffectAddNotify,
        entities: &HashMap<String, EncounterEntity>,
    ) -> Vec<StatusEffectDetails> {
        let timestamp = Utc::now();
        let mut shields: Vec<StatusEffectDetails> = Vec::new();
        for sed in pkt.status_effect_datas {
            let entity = self.get_source_entity(sed.source_id);
            let encounter_entity = entities.get(&entity.name);
            // println!("entity: {:?}", entity);
            let status_effect = build_status_effect(
                sed,
                pkt.character_id,
                entity.id,
                StatusEffectTargetType::Party,
                timestamp,
                encounter_entity,
            );
            if status_effect.status_effect_type == StatusEffectType::Shield {
                shields.push(status_effect.clone());
            }
            self.status_tracker
                .borrow_mut()
                .register_status_effect(status_effect);
        }
        shields
    }
}