use std::vec;

use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{LocalInfo, StatusEffect};
use lost_metrics_sniffer_stub::packets::definitions::PKTPartyInfoInner;
use lost_metrics_sniffer_stub::packets::structures::{StatusEffectData, StatusEffectDataValue};

use super::{NpcTemplate, PartyTemplate, PlayerTemplate, StatusEffectTemplate};

pub struct StateBuilder {
    state: EncounterState
}

impl StateBuilder {
    pub fn new() -> Self {
        Self {
            state: EncounterState::new()
        }
    }

    pub fn set_fight_start(&mut self) {
        self.state.started_on = Utc::now();
    }

    pub fn zero_boss_hp(&mut self) {
        let entities = &mut self.state.entity_stats;
        let boss_entity_stats = entities.get_mut(&self.state.current_boss.as_ref().unwrap().borrow().id).unwrap();
        boss_entity_stats.current_hp = 0;

        // let player_entity_stats = entities.get_mut(&entity_name).unwrap();
        // player_entity_stats.damage_stats.damage_dealt = 1000;
        // self.state.encounter.current_boss_name = boss_name.into();
    }

    pub fn set_damage_stats(&mut self, instance_id: u64, value: i64) {
        self.state.entity_stats.get_mut(&instance_id).unwrap().damage_stats.damage_dealt = value;
    }

    pub fn set_local_player_id(&mut self, id: u64) {
        self.state.local_entity_id = id;
    }

    pub fn set_local_player_name(&mut self, name: String) {
        self.state.local_player_name = Some(name);
    }

    pub fn create_npc(&mut self, template: &NpcTemplate) {
        let now = Utc::now();

        self.state.on_new_npc(
            false,
            now,
            template.object_id,
            template.type_id,
            0,
            template.level,
            template.balance_level,
            template.stat_pairs.to_vec(),
            template.status_effect_datas.to_vec()
        );
    }

    pub fn create_player(&mut self, template: &PlayerTemplate) {
        let now =  Utc::now();

        self.state.new_pc(
            now,
            template.id,
            template.name.to_string(),
            template.class_id,
            template.gear_level,
            template.character_id,
            template.stat_pairs.to_vec(),
            vec![],
            template.status_effect_datas.to_vec());
    }

    pub fn local_player(&mut self, template: &PlayerTemplate) {
        let now =  Utc::now();

        self.state.on_init_pc(
            now,
            template.id,
            template.class_id,
            template.character_id,
            template.name.to_string(),
            template.gear_level,
            template.stat_pairs.to_vec(),
            template.status_effect_datas.to_vec());
    }

    pub fn create_party(&mut self, template: &PartyTemplate) {
        self.state.party_info(
            template.party_instance_id,
            template.raid_instance_id,
            template.members.iter().map(|pr |
                PKTPartyInfoInner {
                    character_id: pr.character_id,
                    class_id: pr.class_id,
                    gear_level: pr.gear_level,
                    name: pr.name.to_string(),
                }
            ).collect(),
            &LocalInfo::default()
        );
    }

    pub fn add_status_effect(&mut self, object_id: u64, status_effect: &StatusEffectTemplate) {
        let now =  Utc::now();
        let data = StatusEffectData {
            source_id: status_effect.source_id,
            stack_count: status_effect.stack_count,
            end_tick: status_effect.end_tick,
            status_effect_id: status_effect.status_effect_id,
            status_effect_instance_id: status_effect.status_effect_instance_id,
            total_time: status_effect.total_time,
            value: StatusEffectDataValue {
                bytearray_0: status_effect.value.clone(),
            }
        };
        self.state.on_status_effect_add(now, object_id, data);
    }

    pub fn add_party_status_effect(&mut self, object_id: u64, status_effect: &StatusEffectTemplate) {
        let now =  Utc::now();
        let datas = vec![
            StatusEffectData {
                source_id: status_effect.source_id,
                stack_count: status_effect.stack_count,
                end_tick: status_effect.end_tick,
                status_effect_id: status_effect.status_effect_id,
                status_effect_instance_id: status_effect.status_effect_instance_id,
                total_time: status_effect.total_time,
                value: StatusEffectDataValue {
                    bytearray_0: status_effect.value.clone(),
                }
            }
        ];
        self.state.on_party_status_effect_add(now, object_id, datas);
    }

    pub fn build(self) -> EncounterState {
        self.state
    }
}
