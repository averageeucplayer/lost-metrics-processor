use std::sync::Arc;

use anyhow::*;
use log::info;
use lost_metrics_core::models::Encounter;
use lost_metrics_store::{encounter_service::EncounterService, models::CreateEncounter};
use tokio::task;

use crate::{{abstractions::AppEvent}, models::CompleteEncounter};

use super::{EventEmitter, StatsApi};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait Persister {
    fn save(&self, version: &str, encounter: CompleteEncounter) -> Result<()>;
}

pub struct DefaultPersister<EE, ES, SA>
where
    EE: EventEmitter,
    ES: EncounterService,
    SA: StatsApi,
{
    stats_api: Arc<SA>,
    encounter_service: Arc<ES>,
    event_emitter: Arc<EE>,
}

impl<EE, ES, SA> DefaultPersister<EE, ES, SA>
where
    EE: EventEmitter,
    ES: EncounterService,
    SA: StatsApi,
{
    pub fn new(
        stats_api: Arc<SA>,
        encounter_service: Arc<ES>,
        event_emitter: Arc<EE>,
    ) -> Self {
        Self {
            stats_api,
            encounter_service,
            event_emitter,
        }
    }
}

impl<EE, ES, SA> Persister for DefaultPersister<EE, ES, SA>
where
    EE: EventEmitter,
    ES: EncounterService,
    SA: StatsApi,
{
    fn save(&self, version: &str, summary: CompleteEncounter) -> Result<()> {
    
        let version = version.to_string();
        let encounter_service = self.encounter_service.clone();
        let event_emitter = self.event_emitter.clone();
        let stats_api = self.stats_api.clone();

        let handle = task::spawn(async move {            

            let create_encounter = CreateEncounter {
                encounter: Encounter {
                    ..Default::default()
                },
                prev_stagger: summary.prev_stagger,
                damage_log: summary.damage_log,
                identity_log: summary.identity_log,
                cast_log: summary.cast_log,
                boss_hp_log: summary.boss_hp_log,
                stagger_log: summary.stagger_log,
                stagger_intervals: summary.stagger_intervals,
                raid_clear: summary.is_raid_clear,
                party_info: summary.party_info,
                raid_difficulty: summary.raid_difficulty,
                region: summary.region,
                version,
                ntp_fight_start: summary.ntp_fight_start,
                rdps_valid: summary.rdps_valid,
                manual: summary.manual,
                skill_cast_log: summary.skill_cast_log,
                player_info: None
            };

            let encounter_id = encounter_service.create(create_encounter)
                .expect("failed to commit transaction");
            info!("saved to db");

            if summary.is_raid_clear {
                event_emitter
                    .emit(AppEvent::ClearEncounter(encounter_id))
                    .expect("failed to emit clear-encounter");
            }
        });

        Ok(())
    }
}