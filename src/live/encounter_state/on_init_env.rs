use lost_metrics_core::models::*;
use lost_metrics_misc::get_class_from_id;
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
        client_id: Option<Uuid>,
        entity: Entity,
        stats_api: Arc<Mutex<SA>>,
        repository: Arc<ES>,
        event_emitter: Arc<EE>) {
        // if not already saved to db, we save again
        if !self.saved && !self.encounter.current_boss_name.is_empty() {
            self.save_to_db(client_id, stats_api, false, repository, event_emitter.clone());
        }

        // replace or insert local player
        if let Some(mut local_player) = self.encounter.entities.remove(&self.encounter.local_player)
        {
            local_player.update(&entity);
            local_player.class = get_class_from_id(&entity.class_id);

            self.encounter
                .entities
                .insert(entity.name.clone(), local_player);
        } else {
            let entity = encounter_entity_from_entity(&entity);
            self.encounter.entities.insert(entity.name.clone(), entity);
        }
        self.encounter.local_player = entity.name;

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