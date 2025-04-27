use lost_metrics_core::models::*;
use lost_metrics_store::encounter_service::EncounterService;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::sync::Arc;
use crate::live::abstractions::EventEmitter;
use crate::live::stats_api::StatsApi;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_init_env<EE : EventEmitter, ES: EncounterService, SA: StatsApi>(
        &mut self,
        version: &str,
        client_id: Option<Uuid>,
        entity: Entity,
        stats_api: Arc<Mutex<SA>>,
        encounter_service: Arc<ES>,
        event_emitter: Arc<EE>) {
        // if not already saved to db, we save again
        if !self.saved && !self.encounter.current_boss_name.is_empty() {
            self.save_to_db(
                version,
                client_id,
                stats_api,
                false,
                encounter_service,
                event_emitter.clone());
        }

        // replace or insert local player
        let entity_name = entity.name.clone();
        if let Some(mut local_player) = self.encounter.entities.remove(&self.encounter.local_player)
        {
            local_player.update(&entity);
            local_player.class = entity.class_id.as_ref().to_string();

            self.encounter
                .entities
                .insert(entity_name.clone(), local_player);
        } else {
            let encounter_entity: EncounterEntity = entity.into();
            self.encounter.entities.insert(entity_name.clone(), encounter_entity);
        }

        self.encounter.local_player = entity_name;

        // remove unrelated entities
        self.encounter.entities.retain(|_, e| {
            e.name == self.encounter.local_player || e.damage_stats.damage_dealt > 0
        });

        event_emitter
            .emit("zone-change", "")
            .expect("failed to emit zone-change");

        self.soft_reset(false);
    }
}