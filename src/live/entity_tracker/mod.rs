mod init_env;
mod init_pc;
mod party_info;
mod party_status_effect_add;
mod party_status_effect_remove;
mod new_npc;
mod new_npc_summon;
mod new_pc;

use crate::live::id_tracker::IdTracker;
use crate::live::party_tracker::PartyTracker;
use crate::live::status_tracker::{
    build_status_effect,
    StatusTracker,
};

use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_data::*;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_sniffer_stub::packets::structures::{NpcStruct, StatPair, StatusEffectData};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct EntityTracker {
    id_tracker: Rc<RefCell<IdTracker>>,
    party_tracker: Rc<RefCell<PartyTracker>>,
    status_tracker: Rc<RefCell<StatusTracker>>,

    pub entities: HashMap<u64, Entity>,

    pub local_entity_id: u64,
    pub local_character_id: u64,
}

impl EntityTracker {
    pub fn new(
        status_tracker: Rc<RefCell<StatusTracker>>,
        id_tracker: Rc<RefCell<IdTracker>>,
        party_tracker: Rc<RefCell<PartyTracker>>,
    ) -> Self {
        Self {
            status_tracker,
            id_tracker,
            party_tracker,
            entities: HashMap::new(),
            local_entity_id: 0,
            local_character_id: 0,
        }
    }

    pub fn new_projectile(&mut self, pkt: &PKTNewProjectile) {
        let projectile = Entity {
            id: pkt.projectile_info.projectile_id,
            entity_type: EntityType::Projectile,
            name: format!("{:x}", pkt.projectile_info.projectile_id),
            owner_id: pkt.projectile_info.owner_id,
            skill_id: pkt.projectile_info.skill_id,
            skill_effect_id: pkt.projectile_info.skill_effect,
            ..Default::default()
        };
        self.entities.insert(projectile.id, projectile);
    }

    pub fn new_trap(&mut self, pkt: &PKTNewTrap) {
        let trap: Entity = Entity {
            id: pkt.trap_struct.object_id,
            entity_type: EntityType::Projectile,
            name: format!("{:x}", pkt.trap_struct.object_id),
            owner_id: pkt.trap_struct.owner_id,
            skill_id: pkt.trap_struct.skill_id,
            skill_effect_id: pkt.trap_struct.skill_effect,
            ..Default::default()
        };
        self.entities.insert(trap.id, trap);
    }

    pub fn get_source_entity(&mut self, id: u64) -> Entity {
        let id = if let Some(entity) = self.entities.get(&id) {
            if entity.entity_type == EntityType::Projectile || entity.entity_type == EntityType::Summon {
                entity.owner_id
            } else {
                id
            }
        } else {
            id
        };

        if let Some(entity) = self.entities.get(&id) {
            entity.clone()
        } else {
            let entity = Entity {
                id,
                entity_type: EntityType::Unknown,
                name: format!("{:x}", id),
                ..Default::default()
            };
            self.entities.insert(entity.id, entity.clone());
            entity
        }
    }

    pub fn id_is_player(&mut self, id: u64) -> bool {
        if let Some(entity) = self.entities.get(&id) {
            entity.entity_type == EntityType::Player
        } else {
            false
        }
    }

    pub fn guess_is_player(&mut self, entity: &mut Entity, skill_id: u32) {
        if (entity.entity_type != EntityType::Unknown && entity.entity_type != EntityType::Player)
            || (entity.entity_type == EntityType::Player && entity.class_id != 0)
        {
            return;
        }

        let class_id = get_skill_class_id(&skill_id);
        if class_id != 0 {
            if entity.entity_type == EntityType::Player {
                if entity.class_id == class_id {
                    return;
                }
                entity.class_id = class_id;
            } else {
                entity.entity_type = EntityType::Player;
                entity.class_id = class_id;
            }
        }
        self.entities.insert(entity.id, entity.clone());
    }

    pub fn build_and_register_status_effect(
        &mut self,
        sed: &StatusEffectData,
        target_id: u64,
        timestamp: DateTime<Utc>,
        entities: Option<&HashMap<String, EncounterEntity>>,
    ) -> StatusEffectDetails {
        let source_entity = self.get_source_entity(sed.source_id);
        let source_encounter_entity =
            entities.and_then(|entities| entities.get(&source_entity.name));
        let status_effect = build_status_effect(
            sed.clone(),
            target_id,
            source_entity.id,
            StatusEffectTargetType::Local,
            timestamp,
            source_encounter_entity,
        );

        self.status_tracker
            .borrow_mut()
            .register_status_effect(status_effect.clone());

        status_effect
    }

    fn build_and_register_status_effects(&mut self, seds: Vec<StatusEffectData>, target_id: u64) {
        let timestamp = Utc::now();
        for sed in seds.into_iter() {
            self.build_and_register_status_effect(&sed, target_id, timestamp, None);
        }
    }

    pub fn get_or_create_entity(&mut self, id: u64) -> Entity {
        if let Some(entity) = self.entities.get(&id) {
            return entity.clone();
        }

        let entity = Entity {
            id,
            entity_type: EntityType::Unknown,
            name: format!("{:x}", id),
            ..Default::default()
        };
        self.entities.insert(entity.id, entity.clone());
        entity
    }

    pub fn get_entity_ref(&self, id: u64) -> Option<&Entity> {
        self.entities.get(&id)
    }
}

pub fn get_skill_class_id(skill_id: &u32) -> u32 {
    if let Some(skill) = SKILL_DATA.get(skill_id) {
        skill.class_id
    } else {
        0
    }
}

fn truncate_gear_level(gear_level: f32) -> f32 {
    f32::trunc(gear_level * 100.) / 100.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_truncate_gear_level() {
        let gear_level = truncate_gear_level(1660.3340);

        assert_eq!(gear_level, 1660.33);
    }
}