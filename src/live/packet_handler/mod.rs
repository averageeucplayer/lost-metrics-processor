mod on_counterattack;
mod on_death;
mod on_identity_change;
mod on_init_env;
mod on_init_pc;
mod on_new_pc;
mod on_new_npc;
mod on_new_npc_summon;
mod on_new_projectile;
mod on_new_trap;
mod on_raid_begin;
mod on_raid_boss_kill;
mod on_raid_result;
mod on_remove_object;
mod on_skill_cast;
mod on_skill_start;
mod on_skill_damage_abnormal;
mod on_skill_damage;
mod on_party_info;
mod on_party_leave;
mod on_party_status_effect_add;
mod on_party_status_effect_remove;
mod on_party_status_effect_result;
mod on_status_effect_add;
mod on_status_effect_remove;
mod on_trigger_boss_battle_status;
mod on_trigger_start;
mod on_zone_member_load;
mod on_zone_object_unpublish;
mod on_status_effect_sync;
mod on_troop_member_update;
mod on_new_transit;
#[cfg(test)]
mod test_utils;

use crate::live::encounter_state::EncounterState;
use crate::live::stats_api::StatsApi;
use crate::live::status_tracker::get_status_effect_value;
use crate::live::utils::get_current_and_max_hp;
use super::trackers::Trackers;
use super::utils::{on_shield_change, parse_pkt};
use super::{abstractions::*, StartOptions};
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::*;
use lost_metrics_data::VALID_ZONES;
use lost_metrics_misc::get_class_from_id;
use lost_metrics_sniffer_stub::decryption::{DamageEncryptionHandler, DamageEncryptionHandlerTrait};
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
use lost_metrics_store::encounter_service::EncounterService;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use super::flags::Flags;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait PacketHandler {
    fn set_damage_handler(&mut self, handler: Box<DamageEncryptionHandler>);
    fn handle(&mut self, opcode: Pkt, data: &[u8], state: &mut EncounterState, options: &StartOptions, rt: Handle) -> anyhow::Result<()>;
}

pub struct DefaultPacketHandler<FL, SA, RS, LP, EE, ES>
where
    FL: Flags,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    ES: EncounterService
{
    trackers: Rc<RefCell<Trackers>>,
    damage_handler: Option<Box<DamageEncryptionHandler>>,
    region_store: Arc<RS>,
    local_player_store: Arc<RwLock<LP>>,
    event_emitter: Arc<EE>,
    repository: Arc<ES>,
    stats_api: Arc<Mutex<SA>>,
    flags: Arc<FL>
}

impl<FL, SA, RS, LP, EE, ES> PacketHandler for DefaultPacketHandler<FL, SA, RS, LP, EE, ES>
where
    FL: Flags,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    ES: EncounterService
{
    fn handle(
        &mut self,
        opcode: Pkt,
        data: &[u8],
        state: &mut EncounterState,
        options: &StartOptions, rt: Handle) -> anyhow::Result<()> {
        let now = Instant::now();

        match opcode {
            Pkt::CounterAttackNotify => self.on_counterattack(data, state)?,
            Pkt::DeathNotify => self.on_death(data, state)?,
            Pkt::IdentityGaugeChangeNotify => self.on_identity_change(data, state)?,
            Pkt::InitEnv => self.on_init_env(data, state)?,
            Pkt::InitPC => self.on_init_pc(data, state)?,
            Pkt::NewPC => {
                if let Some(pkt) = parse_pkt(&data, PKTNewPC::new) {
                    let (hp, max_hp) = get_current_and_max_hp(&pkt.pc_struct.stat_pairs);
                    let entity = self.trackers.borrow_mut().entity_tracker.new_pc(pkt);
                    info!(
                        "new PC: {}, {}, {}, eid: {}, cid: {}",
                        entity.name,
                        get_class_from_id(&entity.class_id),
                        entity.gear_level,
                        entity.id,
                        entity.character_id
                    );
                    state.on_new_pc(entity, hp, max_hp);
                }
            }
            Pkt::NewNpc => {
                if let Some(pkt) = parse_pkt(&data, PKTNewNpc::new) {
                    let (hp, max_hp) = get_current_and_max_hp(&pkt.npc_struct.stat_pairs);
                    let entity = self.trackers.borrow_mut().entity_tracker.new_npc(pkt, max_hp);
                    info!(
                        "new {}: {}, eid: {}, id: {}, hp: {}",
                        entity.entity_type, entity.name, entity.id, entity.npc_id, max_hp
                    );
                    state.on_new_npc(entity, hp, max_hp);
                }
            }
            Pkt::NewNpcSummon => {
                if let Some(pkt) = parse_pkt(&data, PKTNewNpcSummon::new) {
                    let (hp, max_hp) = get_current_and_max_hp(&pkt.npc_struct.stat_pairs);
                    let entity = self.trackers.borrow_mut().entity_tracker.new_npc_summon(pkt, max_hp);
                    info!(
                        "new {}: {}, eid: {}, id: {}, hp: {}",
                        entity.entity_type, entity.name, entity.id, entity.npc_id, max_hp
                    );
                    state.on_new_npc(entity, hp, max_hp);
                }
            }
            Pkt::NewProjectile => {
                if let Some(pkt) = parse_pkt(&data, PKTNewProjectile::new) {
                    self.trackers.borrow_mut().entity_tracker.new_projectile(&pkt);
                    if self.trackers.borrow_mut().entity_tracker.id_is_player(pkt.projectile_info.owner_id)
                        && pkt.projectile_info.skill_id > 0
                    {
                        let key = (pkt.projectile_info.owner_id, pkt.projectile_info.skill_id);
                        if let Some(timestamp) = state.skill_tracker.skill_timestamp.get(&key) {
                            state
                                .skill_tracker
                                .projectile_id_to_timestamp
                                .insert(pkt.projectile_info.projectile_id, timestamp);
                        }
                    }
                }
            }
            Pkt::NewTrap => {
                if let Some(pkt) = parse_pkt(&data, PKTNewTrap::new) {
                    self.trackers.borrow_mut().entity_tracker.new_trap(&pkt);
                    if self.trackers.borrow_mut().entity_tracker.id_is_player(pkt.trap_struct.owner_id)
                        && pkt.trap_struct.skill_id > 0
                    {
                        let key = (pkt.trap_struct.owner_id, pkt.trap_struct.skill_id);
                        if let Some(timestamp) = state.skill_tracker.skill_timestamp.get(&key) {
                            state
                                .skill_tracker
                                .projectile_id_to_timestamp
                                .insert(pkt.trap_struct.object_id, timestamp);
                        }
                    }
                }
            }
            Pkt::RaidBegin => {
                if let Some(pkt) = parse_pkt(&data, PKTRaidBegin::new) {
                    info!("raid begin: {}", pkt.raid_id);
                    match pkt.raid_id {
                        308226 | 308227 | 308239 | 308339 => {
                            state.raid_difficulty = "Trial".to_string();
                            state.raid_difficulty_id = 7;
                        }
                        308428 | 308429 | 308420 | 308410 | 308411 | 308414 | 308422 | 308424
                        | 308421 | 308412 | 308423 | 308426 | 308416 | 308419 | 308415 | 308437
                        | 308417 | 308418 | 308425 | 308430 => {
                            state.raid_difficulty = "Challenge".to_string();
                            state.raid_difficulty_id = 8;
                        }
                        _ => {
                            state.raid_difficulty = "".to_string();
                            state.raid_difficulty_id = 0;
                        }
                    }

                    state.is_valid_zone = VALID_ZONES.contains(&pkt.raid_id);
                }
            }
            Pkt::RaidBossKillNotify => {
                state.on_phase_transition(state.client_id, 1, self.stats_api.clone(), self.repository.clone(), self.event_emitter.clone());
                state.raid_clear = true;
                info!("phase: 1 - RaidBossKillNotify");
            }
            Pkt::RaidResult => {
                state.party_freeze = true;
                state.party_info = if let Some(party) = state.party_cache.take() {
                    party
                } else {
                    state.get_party_from_tracker()
                };
                state.on_phase_transition(state.client_id, 0, self.stats_api.clone(), self.repository.clone(), self.event_emitter.clone());
                state.raid_end_cd = Instant::now();
                info!("phase: 0 - RaidResult");
            }
            Pkt::RemoveObject => {
                if let Some(pkt) = parse_pkt(&data, PKTRemoveObject::new) {
                    for upo in pkt.unpublished_objects {
                        self.trackers.borrow_mut().entity_tracker.entities.remove(&upo.object_id);
                        self.trackers.borrow().status_tracker
                            .borrow_mut()
                            .remove_local_object(upo.object_id);
                    }
                }
            }
            Pkt::SkillCastNotify => {
                if let Some(pkt) = parse_pkt(&data, PKTSkillCastNotify::new) {
                    let mut entity = self.trackers.borrow_mut().entity_tracker.get_source_entity(pkt.source_id);
                    self.trackers.borrow_mut().entity_tracker.guess_is_player(&mut entity, pkt.skill_id);
                    if entity.class_id == 202 {
                        state.on_skill_start(
                            &entity,
                            pkt.skill_id,
                            None,
                            None,
                            Utc::now().timestamp_millis(),
                        );
                    }
                }
            }
            Pkt::SkillStartNotify => {
                if let Some(pkt) = parse_pkt(&data, PKTSkillStartNotify::new)
                {
                    let mut entity = self.trackers.borrow_mut().entity_tracker.get_source_entity(pkt.source_id);
                    self.trackers.borrow_mut().entity_tracker.guess_is_player(&mut entity, pkt.skill_id);
                    let tripod_index =
                        pkt.skill_option_data
                            .tripod_index
                            .map(|tripod_index| lost_metrics_core::models::TripodIndex {
                                first: tripod_index.first,
                                second: tripod_index.second,
                                third: tripod_index.third,
                            });
                    let tripod_level =
                        pkt.skill_option_data
                            .tripod_level
                            .map(|tripod_level| lost_metrics_core::models::TripodLevel {
                                first: tripod_level.first,
                                second: tripod_level.second,
                                third: tripod_level.third,
                            });
                    let timestamp = Utc::now().timestamp_millis();
                    let (skill_id, summon_source) = state.on_skill_start(
                        &entity,
                        pkt.skill_id,
                        tripod_index,
                        tripod_level,
                        timestamp,
                    );

                    if entity.entity_type == EntityType::Player && skill_id > 0 {
                        state
                            .skill_tracker
                            .new_cast(entity.id, skill_id, summon_source, timestamp);
                    }
                }
            }
            Pkt::SkillDamageAbnormalMoveNotify => {
                if now - state.raid_end_cd < options.raid_end_capture_timeout {
                    info!("ignoring damage - SkillDamageAbnormalMoveNotify");
                    return Ok(());
                }
                if let Some(pkt) = parse_pkt(
                    &data,
                    PKTSkillDamageAbnormalMoveNotify::new
                ) {
                    let now = Utc::now().timestamp_millis();
                    let owner = self.trackers.borrow_mut().entity_tracker.get_source_entity(pkt.source_id);
                    let local_character_id = self.trackers.borrow().id_tracker
                        .borrow()
                        .get_local_character_id(self.trackers.borrow().entity_tracker.local_entity_id);
                    let target_count = pkt.skill_damage_abnormal_move_events.len() as i32;

                    let player_stats = if state.is_valid_zone {
                        rt.block_on(async {
                            self.stats_api.lock().await.get_stats(&state)
                        })
                    }
                    else {
                        None
                    };

                    for mut event in pkt.skill_damage_abnormal_move_events.into_iter() {
                        if !self.damage_handler.as_ref().unwrap().decrypt_damage_event(&mut event.skill_damage_event) {
                            state.damage_is_valid = false;
                            continue;
                        }
                        let target_entity =
                            self.trackers.borrow_mut().entity_tracker.get_or_create_entity(event.skill_damage_event.target_id);
                        let source_entity = self.trackers.borrow_mut().entity_tracker.get_or_create_entity(pkt.source_id);

                        // track potential knockdown
                        state.on_abnormal_move(&target_entity, &event.skill_move_option_data, now);

                        let (se_on_source, se_on_target) = self.trackers.borrow().status_tracker
                            .borrow_mut()
                            .get_status_effects(&owner, &target_entity, local_character_id);
                        let damage_data = DamageData {
                            skill_id: pkt.skill_id,
                            skill_effect_id: pkt.skill_effect_id,
                            damage: event.skill_damage_event.damage,
                            modifier: event.skill_damage_event.modifier as i32,
                            target_current_hp: event.skill_damage_event.cur_hp,
                            target_max_hp: event.skill_damage_event.max_hp,
                            damage_attribute: event.skill_damage_event.damage_attr,
                            damage_type: event.skill_damage_event.damage_type,
                        };

                        state.on_damage(
                            &owner,
                            &source_entity,
                            &target_entity,
                            damage_data,
                            se_on_source,
                            se_on_target,
                            target_count,
                            now,
                            self.event_emitter.clone()
                        );
                    }
                }
            }
            Pkt::SkillDamageNotify => {
                // use this to make sure damage packets are not tracked after a raid just wiped
                if now - state.raid_end_cd < options.raid_end_capture_timeout {
                    info!("ignoring damage - SkillDamageNotify");
                    return Ok(());
                }
                if let Some(pkt) =
                    parse_pkt(&data, PKTSkillDamageNotify::new)
                {
                    let now = Utc::now().timestamp_millis();
                    let owner = self.trackers.borrow_mut().entity_tracker.get_source_entity(pkt.source_id);
                    let local_character_id = self.trackers.borrow().id_tracker
                        .borrow()
                        .get_local_character_id(self.trackers.borrow().entity_tracker.local_entity_id);
                    let target_count = pkt.skill_damage_events.len() as i32;
    
                    let player_stats = if state.is_valid_zone {
                        rt.block_on(async {
                            self.stats_api.lock().await.get_stats(&state)
                        })
                    }
                    else {
                        None
                    };

                    for mut event in pkt.skill_damage_events.into_iter() {
                        if !self.damage_handler.as_ref().unwrap().decrypt_damage_event(&mut event) {
                            state.damage_is_valid = false;
                            continue;
                        }
                        let target_entity = self.trackers.borrow_mut().entity_tracker.get_or_create_entity(event.target_id);
                        // source_entity is to determine battle item
                        let source_entity = self.trackers.borrow_mut().entity_tracker.get_or_create_entity(pkt.source_id);
                        let (se_on_source, se_on_target) = self.trackers.borrow().status_tracker
                            .borrow_mut()
                            .get_status_effects(&owner, &target_entity, local_character_id);
                        let damage_data = DamageData {
                            skill_id: pkt.skill_id,
                            skill_effect_id: pkt.skill_effect_id.unwrap_or_default(),
                            damage: event.damage,
                            modifier: event.modifier as i32,
                            target_current_hp: event.cur_hp,
                            target_max_hp: event.max_hp,
                            damage_attribute: event.damage_attr,
                            damage_type: event.damage_type,
                        };
                        state.on_damage(
                            &owner,
                            &source_entity,
                            &target_entity,
                            damage_data,
                            se_on_source,
                            se_on_target,
                            target_count,
                            now,
                            self.event_emitter.clone()
                        );
                    }
                }
            }
            Pkt::PartyInfo => {
                if let Some(pkt) = parse_pkt(&data, PKTPartyInfo::new) {
                    let local_player_store = self.local_player_store.read().unwrap();
                    let local_info = local_player_store.get();
                    self.trackers.borrow_mut().entity_tracker.party_info(pkt, local_info);
                    let local_player_id = self.trackers.borrow().entity_tracker.local_entity_id;
                    if let Some(entity) = self.trackers.borrow().entity_tracker.entities.get(&local_player_id) {
                        state.update_local_player(entity);
                    }
                    state.party_cache = None;
                    state.party_map_cache = HashMap::new();
                }
            }
            Pkt::PartyLeaveResult => {
                if let Some(pkt) = parse_pkt(&data, PKTPartyLeaveResult::new)
                {
                    self.trackers.borrow().party_tracker
                        .borrow_mut()
                        .remove(pkt.party_instance_id, pkt.name);
                    state.party_cache = None;
                    state.party_map_cache = HashMap::new();
                }
            }
            Pkt::PartyStatusEffectAddNotify => {
                if let Some(pkt) = parse_pkt(
                    &data,
                    PKTPartyStatusEffectAddNotify::new) {
                    // info!("{:?}", pkt);
                    let shields =
                        self.trackers.borrow_mut().entity_tracker.party_status_effect_add(pkt, &state.encounter.entities);
                    for status_effect in shields {
                        let source = self.trackers.borrow_mut().entity_tracker.get_source_entity(status_effect.source_id);
                        let target_id =
                            if status_effect.target_type == StatusEffectTargetType::Party {
                                self.trackers.borrow().id_tracker
                                    .borrow()
                                    .get_entity_id(status_effect.target_id)
                                    .unwrap_or_default()
                            } else {
                                status_effect.target_id
                            };
                        let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(target_id);
                        // info!("SHIELD SOURCE: {} > TARGET: {}", source.name, target.name);
                        state.on_boss_shield(&target, status_effect.value);
                        state.on_shield_applied(
                            &source,
                            &target,
                            status_effect.status_effect_id,
                            status_effect.value,
                        );
                    }
                }
            }
            Pkt::PartyStatusEffectRemoveNotify => {
                if let Some(pkt) = parse_pkt(
                    &data,
                    PKTPartyStatusEffectRemoveNotify::new) {
                    let (is_shield, shields_broken, _effects_removed, _left_workshop) =
                        self.trackers.borrow_mut().entity_tracker.party_status_effect_remove(pkt);
                    if is_shield {
                        for status_effect in shields_broken {
                            let change = status_effect.value;
                            on_shield_change(
                                &mut self.trackers.borrow_mut().entity_tracker,
                                &self.trackers.borrow().id_tracker,
                                state,
                                status_effect,
                                change,
                            );
                        }
                    }
                }
            }
            Pkt::PartyStatusEffectResultNotify => {
                if let Some(pkt) = parse_pkt(
                    &data, PKTPartyStatusEffectResultNotify::new) {
                    // info!("{:?}", pkt);
                    self.trackers.borrow().party_tracker.borrow_mut().add(
                        pkt.raid_instance_id,
                        pkt.party_instance_id,
                        pkt.character_id,
                        0,
                        None,
                    );
                }
            }
            Pkt::StatusEffectAddNotify => {
                if let Some(pkt) = parse_pkt(
                    &data,
                    PKTStatusEffectAddNotify::new) {
                    let status_effect = self.trackers.borrow_mut().entity_tracker.build_and_register_status_effect(
                        &pkt.status_effect_data,
                        pkt.object_id,
                        Utc::now(),
                        Some(&state.encounter.entities),
                    );

                    if status_effect.status_effect_type == StatusEffectType::Shield {
                        let source = self.trackers.borrow_mut().entity_tracker.get_source_entity(status_effect.source_id);
                        let target_id =
                            if status_effect.target_type == StatusEffectTargetType::Party {
                                self.trackers.borrow().id_tracker
                                    .borrow()
                                    .get_entity_id(status_effect.target_id)
                                    .unwrap_or_default()
                            } else {
                                status_effect.target_id
                            };
                        let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(target_id);
                        state.on_boss_shield(&target, status_effect.value);
                        state.on_shield_applied(
                            &source,
                            &target,
                            status_effect.status_effect_id,
                            status_effect.value,
                        );
                    }

                    if status_effect.status_effect_type == StatusEffectType::HardCrowdControl {
                        let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(status_effect.target_id);
                        if target.entity_type == EntityType::Player {
                            state.on_cc_applied(&target, &status_effect);
                        }
                    }
                }
            }
            Pkt::StatusEffectRemoveNotify => {
                if let Some(pkt) = parse_pkt(
                    &data, PKTStatusEffectRemoveNotify::new) {
                    let (is_shield, shields_broken, effects_removed, _left_workshop) =
                        self.trackers.borrow().status_tracker.borrow_mut().remove_status_effects(
                            pkt.object_id,
                            pkt.status_effect_instance_ids,
                            pkt.reason,
                            StatusEffectTargetType::Local,
                        );
                    if is_shield {
                        if shields_broken.is_empty() {
                            let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(pkt.object_id);
                            state.on_boss_shield(&target, 0);
                        } else {
                            for status_effect in shields_broken {
                                let change = status_effect.value;
                                on_shield_change(
                                    &mut self.trackers.borrow_mut().entity_tracker,
                                    &self.trackers.borrow().id_tracker,
                                    state,
                                    status_effect,
                                    change,
                                );
                            }
                        }
                    }
                    let now = Utc::now().timestamp_millis();
                    for effect_removed in effects_removed {
                        if effect_removed.status_effect_type == StatusEffectType::HardCrowdControl {
                            let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(effect_removed.target_id);
                            if target.entity_type == EntityType::Player {
                                state.on_cc_removed(&target, &effect_removed, now);
                            }
                        }
                    }
                }
            }
            Pkt::TriggerBossBattleStatus => {
                // need to hard code clown because it spawns before the trigger is sent???
                if state.encounter.current_boss_name.is_empty()
                    || state.encounter.fight_start == 0
                    || state.encounter.current_boss_name == "Saydon"
                {
                    state.on_phase_transition(state.client_id, 3, self.stats_api.clone(), self.repository.clone(), self.event_emitter.clone());
                    info!("phase: 3 - resetting encounter - TriggerBossBattleStatus");
                }
            }
            Pkt::TriggerStartNotify => {
                if let Some(pkt) =
                    parse_pkt(&data, PKTTriggerStartNotify::new)
                {
                    match pkt.signal {
                        57 | 59 | 61 | 63 | 74 | 76 => {
                            state.party_freeze = true;
                            state.party_info = if let Some(party) = state.party_cache.take() {
                                party
                            } else {
                                state.get_party_from_tracker()
                            };
                            state.raid_clear = true;
                            state.on_phase_transition(state.client_id, 2, self.stats_api.clone(), self.repository.clone(), self.event_emitter.clone());
                            state.raid_end_cd = Instant::now();
                            info!("phase: 2 - clear - TriggerStartNotify");
                        }
                        58 | 60 | 62 | 64 | 75 | 77 => {
                            state.party_freeze = true;
                            state.party_info = if let Some(party) = state.party_cache.take() {
                                party
                            } else {
                                state.get_party_from_tracker()
                            };
                            state.raid_clear = false;
                            state.on_phase_transition(state.client_id, 4, self.stats_api.clone(), self.repository.clone(), self.event_emitter.clone());
                            state.raid_end_cd = Instant::now();
                            info!("phase: 4 - wipe - TriggerStartNotify");
                        }
                        27 | 10 | 11 => {
                            // debug_print(format_args!("old rdps sync time - {}", pkt.trigger_signal_type));
                        }
                        _ => {}
                    }
                }
            }
            Pkt::ZoneMemberLoadStatusNotify => {
                if let Some(pkt) = parse_pkt(
                    &data,
                    PKTZoneMemberLoadStatusNotify::new) {
                    state.is_valid_zone = VALID_ZONES.contains(&pkt.zone_id);

                    if state.raid_difficulty_id >= pkt.zone_id && !state.raid_difficulty.is_empty()
                    {
                        return Ok(());
                    }
                    
                    info!("raid zone id: {}", &pkt.zone_id);
                    info!("raid zone id: {}", &pkt.zone_level);

                    match pkt.zone_level {
                        0 => {
                            state.raid_difficulty = "Normal".to_string();
                            state.raid_difficulty_id = 0;
                        }
                        1 => {
                            state.raid_difficulty = "Hard".to_string();
                            state.raid_difficulty_id = 1;
                        }
                        2 => {
                            state.raid_difficulty = "Inferno".to_string();
                            state.raid_difficulty_id = 2;
                        }
                        3 => {
                            state.raid_difficulty = "Challenge".to_string();
                            state.raid_difficulty_id = 3;
                        }
                        4 => {
                            state.raid_difficulty = "Solo".to_string();
                            state.raid_difficulty_id = 4;
                        }
                        5 => {
                            state.raid_difficulty = "The First".to_string();
                            state.raid_difficulty_id = 5;
                        }
                        _ => {}
                    }
                }
            }
            Pkt::ZoneObjectUnpublishNotify => {
                if let Some(pkt) = parse_pkt(&data, PKTZoneObjectUnpublishNotify::new) {
                    self.trackers.borrow().status_tracker
                        .borrow_mut()
                        .remove_local_object(pkt.object_id);
                }
            }
            Pkt::StatusEffectSyncDataNotify => {
                if let Some(pkt) = parse_pkt(&data, PKTStatusEffectSyncDataNotify::new) {
                    let (status_effect, old_value) =
                        self.trackers.borrow().status_tracker.borrow_mut().sync_status_effect(
                            pkt.status_effect_instance_id,
                            pkt.character_id,
                            pkt.object_id,
                            pkt.value,
                            self.trackers.borrow().entity_tracker.local_character_id,
                        );
                    if let Some(status_effect) = status_effect {
                        if status_effect.status_effect_type == StatusEffectType::Shield {
                            let change = old_value
                                .checked_sub(status_effect.value)
                                .unwrap_or_default();
                            on_shield_change(
                                &mut self.trackers.borrow_mut().entity_tracker,
                                &self.trackers.borrow().id_tracker,
                                state,
                                status_effect,
                                change,
                            );
                        }
                    }
                }
            }
            Pkt::TroopMemberUpdateMinNotify => {
                if let Some(pkt) = parse_pkt(
                    &data, PKTTroopMemberUpdateMinNotify::new) {
                    // info!("{:?}", pkt);
                    if let Some(object_id) = self.trackers.borrow().id_tracker.borrow().get_entity_id(pkt.character_id) {
                        if let Some(entity) = self.trackers.borrow().entity_tracker.get_entity_ref(object_id) {
                            state
                                .encounter
                                .entities
                                .entry(entity.name.clone())
                                .and_modify(|e| {
                                    e.current_hp = pkt.cur_hp;
                                    e.max_hp = pkt.max_hp;
                                });
                        }
                        for se in pkt.status_effect_datas.iter() {
                            let val = get_status_effect_value(&se.value);
                            let (status_effect, old_value) =
                                self.trackers.borrow().status_tracker.borrow_mut().sync_status_effect(
                                    se.status_effect_instance_id,
                                    pkt.character_id,
                                    object_id,
                                    val,
                                    self.trackers.borrow().entity_tracker.local_character_id,
                                );
                            if let Some(status_effect) = status_effect {
                                if status_effect.status_effect_type == StatusEffectType::Shield {
                                    let change = old_value
                                        .checked_sub(status_effect.value)
                                        .unwrap_or_default();
                                    on_shield_change(
                                        &mut self.trackers.borrow_mut().entity_tracker,
                                        &self.trackers.borrow().id_tracker,
                                        state,
                                        status_effect,
                                        change,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Pkt::NewTransit => self.on_new_transit(data)?,
            _ => {}
        }

        Ok(())
    }
    
    fn set_damage_handler(&mut self, handler: Box<DamageEncryptionHandler>) {
        self.damage_handler = Some(handler);
    }
}

impl<FL, SA, RS, LP, EE, ES> DefaultPacketHandler<FL, SA, RS, LP, EE, ES>
where
    FL: Flags,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    ES: EncounterService {
    pub fn new(
        flags: Arc<FL>,
        trackers: Rc<RefCell<Trackers>>,
        local_player_store: Arc<RwLock<LP>>,
        event_emitter: Arc<EE>,
        region_store: Arc<RS>,
        repository: Arc<ES>,
        stats_api: Arc<Mutex<SA>>) -> Self {
        

        Self {
            flags,
            local_player_store,
            damage_handler: None,
            event_emitter,
            region_store,
            repository,
            trackers,
            stats_api,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, env, rc::Rc, sync::{Arc, RwLock}, time::Duration, vec};
    use lost_metrics_sniffer_stub::packets::{definitions::{PKTCounterAttackNotify, PKTNewPC, PKTNewPCInner}, opcodes::Pkt};
    use tokio::{runtime::Handle, sync::Mutex};
    use crate::live::{abstractions::*, encounter_state::EncounterState, entity_tracker::EntityTracker, flags::MockFlags, id_tracker::IdTracker, packet_handler::{test_utils::create_random_pc, DefaultPacketHandler, PacketHandler}, party_tracker::PartyTracker, stats_api::MockStatsApi, status_tracker::StatusTracker, test_utils::create_start_options, trackers::Trackers, StartOptions};


    #[tokio::test]
    async fn test() {

      
    }
}