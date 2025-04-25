use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_misc::*;
use lost_metrics_store::encounter_service::EncounterService;
use lost_metrics_store::models::CreateEncounter;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::sync::Arc;
use tokio::task;
use crate::live::abstractions::EventEmitter;
use crate::live::stats_api::StatsApi;

use super::EncounterState;

impl EncounterState {
    pub fn save_to_db<EE: EventEmitter, ES: EncounterService, SA: StatsApi>(
        &mut self,
        version: &str,
        client_id: Option<Uuid>,
        stats_api: Arc<Mutex<SA>>,
        manual: bool,
        encounter_service: Arc<ES>,
        event_emitter: Arc<EE>
        ) {
        let entities = &self.encounter.entities;

        if !manual {
            if self.encounter.fight_start == 0
                || self.encounter.current_boss_name.is_empty()
                || !entities.contains_key(&self.encounter.current_boss_name)
                || !entities.values().any(|e| e.entity_type == EntityType::Player && e.damage_stats.damage_dealt > 0)
            {
                return;
            }

            if let Some(current_boss) = entities.get(&self.encounter.current_boss_name)
            {
                if current_boss.current_hp == current_boss.max_hp {
                    return;
                }
            }
        }

        if !self.damage_is_valid {
            warn!("damage decryption is invalid, not saving to db");
        }

        let mut encounter = self.encounter.clone();
        let prev_stagger = self.prev_stagger;
        let damage_log = self.damage_log.clone();
        let identity_log = self.identity_log.clone();
        let cast_log = self.cast_log.clone();
        let boss_hp_log = self.boss_hp_log.clone();
        let stagger_log = self.stagger_log.clone();
        let stagger_intervals = self.stagger_intervals.clone();
        let raid_clear = self.raid_clear;
        let party_info = self.party_info.clone();
        let raid_difficulty = self.raid_difficulty.clone();
        let region = self.region.clone();
        let ntp_fight_start = self.ntp_fight_start;
        let rdps_valid = self.rdps_valid;
        let skill_cast_log = self.get_cast_log();
        let version = version.to_string();
        
        info!(
            "saving to db - cleared: [{}], difficulty: [{}] {}",
            raid_clear, self.raid_difficulty, encounter.current_boss_name
        );

        encounter.current_boss_name = update_current_boss_name(&encounter.current_boss_name);

        task::spawn(async move {
            let stats_api = stats_api.lock().await;

            let player_info = if !raid_difficulty.is_empty()
                && !encounter.current_boss_name.is_empty()
            {
                info!("fetching player info");
                let players = encounter
                    .entities
                    .values()
                    .filter(|entity| entity.is_valid_player())
                    .map(|entity| entity.name.clone())
                    .collect::<Vec<_>>();

                let valid_party = !players.is_empty() && players.len() <= 16;

                if let Some(client_id) = client_id.filter(|_| valid_party && region.is_some()) {
                    stats_api
                        .get_character_info(client_id, &encounter.current_boss_name, players, region.clone())
                        .await
                } else {
                    None
                }
            } else {
                None
            };

            let create_encounter = CreateEncounter {
                encounter,
                prev_stagger,
                damage_log,
                identity_log,
                cast_log,
                boss_hp_log,
                stagger_log,
                stagger_intervals,
                raid_clear,
                party_info,
                raid_difficulty,
                region,
                player_info,
                version,
                ntp_fight_start,
                rdps_valid,
                manual,
                skill_cast_log,
            };

            let encounter_id = encounter_service.create(create_encounter)
                .expect("failed to commit transaction");
            info!("saved to db");

            if raid_clear {
                event_emitter
                    .emit("clear-encounter", encounter_id)
                    .expect("failed to emit clear-encounter");
            }
        });
    }
}