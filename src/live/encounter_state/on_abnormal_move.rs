use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_sniffer_stub::packets::common::SkillMoveOptionData;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_abnormal_move(
        &mut self,
        victim_entity: &Entity,
        movement: &SkillMoveOptionData,
        timestamp: i64,
    ) {
        if victim_entity.entity_type != EntityType::Player {
            // we don't care about npc knockups
            return;
        }

        // only count movement events that would result in a knockup
        let Some(down_time) = movement.down_time else {
            return;
        };

        // todo: unclear if this is fully correct. It's hard to debug this, but it seems roughly accurate
        // if this is not accurate, we should probably factor out the stand_up_time and instead add in the
        // animation duration of the standup action for each class (seems to be 0.9s)
        let total_incapacitated_time = down_time
            + movement.move_time.unwrap_or_default()
            + movement.stand_up_time.unwrap_or_default();
        let incapacitated_time_ms = (total_incapacitated_time * 1000.0) as i64;

        let victim_entity_state = self
            .encounter
            .entities
            .entry(victim_entity.name.clone())
            .or_insert_with(|| encounter_entity_from_entity(victim_entity));

        // see if we have a previous incapacitation event that is still in effect (i.e. the player was knocked up again before
        // they could stand up), in which case we should shorten the previous event duration to the current timestamp
        let prev_incapacitation = victim_entity_state
            .damage_stats
            .incapacitations
            .iter_mut()
            .rev()
            .take_while(|e| e.timestamp + e.duration > timestamp) // stop as soon as we only hit expired events
            .find(|x| x.event_type == IncapacitationEventType::FallDown); // find an unexpired one that was caused by an abnormal move
        if let Some(prev_incapacitation) = prev_incapacitation {
            info!(
                "Shortening down duration from {} to {} because of new abnormal move",
                prev_incapacitation.duration,
                timestamp - prev_incapacitation.timestamp
            );
            prev_incapacitation.duration = timestamp - prev_incapacitation.timestamp;
        }

        let new_event = IncapacitatedEvent {
            timestamp,
            duration: incapacitated_time_ms,
            event_type: IncapacitationEventType::FallDown,
        };
        victim_entity_state
            .damage_stats
            .incapacitations
            .push(new_event);
        info!(
            "Player {} will be incapacitated for {}ms",
            victim_entity_state.name, incapacitated_time_ms
        );
    }

}