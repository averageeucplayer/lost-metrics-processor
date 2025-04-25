use std::vec;

use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
use log::*;

use super::{NpcTemplate, PlayerTemplate};

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
        self.state.encounter.fight_start = Utc::now().timestamp_millis();
    }

    pub fn zero_boss_hp(&mut self) {
        let entities = &mut self.state.encounter.entities;
        let boss_entity_stats = entities.get_mut(&self.state.encounter.current_boss_name).unwrap();
        boss_entity_stats.current_hp = 0;

        // let player_entity_stats = entities.get_mut(&entity_name).unwrap();
        // player_entity_stats.damage_stats.damage_dealt = 1000;
        // self.state.encounter.current_boss_name = boss_name.into();
    }

    pub fn set_damage_stats(&mut self, value: u64) {

    }

    pub fn set_local_player_id(&mut self, id: u64) {
        self.state.local_entity_id = id;
    }

    pub fn set_local_player_name(&mut self, name: String) {
        self.state.encounter.local_player = name;
    }

    pub fn create_npc(&mut self, npc: &NpcTemplate) {

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

    pub fn build(self) -> EncounterState {
        self.state
    }
}
