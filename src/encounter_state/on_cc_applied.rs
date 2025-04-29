use log::{info, warn};
use lost_metrics_core::models::*;
use crate::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_cc_applied(&mut self, victim_entity: &Entity, status_effect: &StatusEffectDetails) {
        let victim_entity_state = self
            .entity_stats
            .entry(victim_entity.id)
            .or_insert_with(|| encounter_entity_from_entity(victim_entity));

        // expiration delay is zero or negative for infinite effects. Instead of applying them now,
        // only apply them after they've been removed (this avoids an issue where if we miss the removal
        // we end up applying a very long incapacitation)
        if status_effect.is_infinite() {
            return;
        }

        let duration_ms = status_effect.expiration_delay * 1000.0;
        let new_event = IncapacitatedEvent {
            timestamp: status_effect.timestamp.timestamp_millis(),
            duration: duration_ms as i64,
            event_type: IncapacitationEventType::CrowdControl,
        };
        info!(
            "Player {} will be status-effect incapacitated for {}ms by buff {}",
            victim_entity_state.name, duration_ms, status_effect.status_effect_id
        );
        victim_entity_state
            .damage_stats
            .incapacitations
            .push(new_event);
    }

}