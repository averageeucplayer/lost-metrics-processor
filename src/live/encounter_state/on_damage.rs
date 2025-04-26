use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use lost_metrics_core::models::*;
use lost_metrics_misc::*;
use lost_metrics_sniffer_stub::packets::structures::SkillDamageEvent;
use core::time;
use std::cmp::max;
use std::default::Default;
use std::sync::Arc;

use crate::live::abstractions::EventEmitter;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {
    
    pub fn on_damage_agg(
        &mut self,
        now: DateTime<Utc>,
        source_id: u64,
        valid_events: Vec<bool>,
        skill_damage_events: Vec<SkillDamageEvent>,
        skill_id: u32,
        skill_effect_id: Option<u32>,
    ) -> DamageResult {
        let now = now.timestamp_millis();
        let local_character_id = self.entity_id_to_character_id
            .get(&self.local_entity_id)
            .copied()
            .unwrap_or_default();

        // source_entity is to determine battle item
        let owner_entity = self.get_source_entity(source_id).clone();
        let source_entity = self.get_or_create_entity(source_id).clone();;

        let processed: Vec<_> = skill_damage_events
            .into_iter()
            .map(|event| {
                let hit_flag: HitFlag = (event.modifier & 0xf).into();
                let hit_option: HitOption = ((event.modifier >> 4) & 0x7).into();
                let target_entity = self.get_or_create_entity(event.target_id).clone();

                let (se_on_source, se_on_target) = self.get_status_effects(&source_entity, &target_entity, local_character_id);

                for se in se_on_source.iter() {
                    if se.custom_id > 0 {
                        self.custom_id_map.insert(se.custom_id, se.status_effect_id);
                    }
                }

                for se in se_on_target.iter() {
                    if se.custom_id > 0 {
                        self.custom_id_map.insert(se.custom_id, se.status_effect_id);
                    }
                }

                let se_on_source_ids: Vec<u32> = se_on_source
                    .iter()
                    .map(|se| if se.custom_id > 0 { se.custom_id } else { se.status_effect_id })
                    .collect();

                let se_on_target_ids: Vec<u32> = se_on_source
                    .iter()
                    .map(|se| if se.custom_id > 0 { se.custom_id } else { se.status_effect_id })
                    .collect();

                let mut damage_event = DamageEvent {
                    is_valid: true,
                    is_battle_item: false,
                    hit_flag,
                    hit_option,
                    skill_id: skill_id,
                    owner_entity: &owner_entity,
                    source_entity: &source_entity,
                    skill_effect_id: skill_effect_id.clone(),
                    damage: event.damage,
                    se_on_source,
                    se_on_source_ids,
                    se_on_target_ids,
                    se_on_target,
                    target_current_hp: event.cur_hp,
                    target_max_hp: event.max_hp,
                    target_entity,
                    timestamp: now
                };

                if damage_event.hit_flag == HitFlag::Invincible {
                    damage_event.is_valid = false;
                    return damage_event;
                }

                if damage_event.hit_flag == HitFlag::DamageShare
                    && skill_id == 0
                    && skill_effect_id.is_none()
                {
                    damage_event.is_valid = false;
                    return damage_event;
                }

                if source_entity.entity_type == EntityType::Projectile
                    && is_battle_item(&source_entity.skill_effect_id, "attack")
                {
                    damage_event.is_battle_item = true;
                    damage_event.skill_effect_id = Some(source_entity.skill_effect_id);
                }

                // if boss only damage is enabled
                // check if target is boss and not player
                // check if target is player and source is boss
                if self.boss_only_damage
                && ((damage_event.target_entity.entity_type != EntityType::Boss
                    && damage_event.target_entity.entity_type != EntityType::Player)
                    || (damage_event.target_entity.entity_type == EntityType::Player
                        && damage_event.source_entity.entity_type != EntityType::Boss))
                {
                    damage_event.is_valid = false;
                    return damage_event;
                }

                damage_event
            }).collect();
      

        let mut result = DamageResult {
            is_raid_start: false
        };

        for mut event in processed.into_iter().filter(|pr| pr.is_valid) {
            
            let result_it = self.on_damage(event);
            result.is_raid_start = result.is_raid_start || result_it.is_raid_start;
        }

        result
    }

    pub fn on_damage(&mut self, event: DamageEvent) -> DamageResult {
   
        let DamageEvent {
            owner_entity,
            target_entity,
            source_entity,
            damage,
            hit_flag,
            hit_option,
            is_battle_item,
            is_valid,
            skill_effect_id,
            skill_id,
            target_current_hp,
            target_max_hp,
            se_on_source,
            se_on_source_ids,
            se_on_target,
            se_on_target_ids,
            timestamp
        } = event;

        let skill_effect_id = skill_effect_id.unwrap_or_default();

        let mut result = DamageResult {
            is_raid_start: false
        };
      
        let entities = &mut self.encounter.entities;
        let entities_ptr: *mut hashbrown::HashMap<String, EncounterEntity> = entities as *mut _;

        let source_encounter_entity = unsafe {
            (*entities_ptr)
                .entry(owner_entity.name.clone())
                .or_insert_with(|| encounter_entity_from_entity(owner_entity))
        };

        let target_encounter_entity = unsafe {
            (*entities_ptr)
                .entry(target_entity.name.clone())
                .or_insert_with(|| {
                    let mut target_entity = encounter_entity_from_entity(&target_entity);
                    target_entity.current_hp = target_current_hp;
                    target_entity.max_hp = target_max_hp;
                    target_entity
                })
        };

        if self.encounter.fight_start == 0 {
            self.encounter.fight_start = timestamp;
            self.fight_start = timestamp;
            if source_encounter_entity.entity_type == EntityType::Player && skill_id > 0 {
                self.new_cast(
                    source_encounter_entity.id,
                    skill_id,
                    None,
                    timestamp,
                );
            }

            if let Ok(result) = self.sntp_client.synchronize("time.cloudflare.com") {
                let dt = result.datetime().into_chrono_datetime().unwrap_or_default();
                self.ntp_fight_start = dt.timestamp_millis();
                // debug_print(format_args!("fight start local: {}, ntp: {}", Utc::now().to_rfc3339(), dt.to_rfc3339()));
            };

            self.encounter.boss_only_damage = self.boss_only_damage;

            result.is_raid_start = true;
        }

        self.encounter.last_combat_packet = timestamp;

        source_encounter_entity.id = owner_entity.id;

        if target_entity.id == owner_entity.id {
            target_encounter_entity.current_hp = target_current_hp;
            target_encounter_entity.max_hp = target_max_hp;
        }

        let mut damage = damage;
        if target_entity.entity_type != EntityType::Player && target_current_hp < 0 {
            damage += target_current_hp;
        }

        let mut skill_id = if skill_id != 0 {
            skill_id
        } else {
            skill_effect_id
        };

        let skill_data = get_skill(&skill_id);
        let mut skill_name = "".to_string();
        let mut skill_summon_sources: Option<Vec<u32>> = None;
        if let Some(skill_data) = skill_data.as_ref() {
            skill_name = skill_data.name.clone().unwrap_or_default();
            skill_summon_sources.clone_from(&skill_data.summon_source_skills);
        }

        if skill_name.is_empty() {
            (skill_name, _, skill_summon_sources) = get_skill_name_and_icon(
                &skill_id,
                &skill_effect_id,
                skill_id.to_string(),
                &self.skill_timestamp,
                source_encounter_entity.id,
            );
        }
        let relative_timestamp = (timestamp - self.encounter.fight_start) as i32;

        if !source_encounter_entity.skills.contains_key(&skill_id) {
            if let Some(skill) = source_encounter_entity
                .skills
                .values()
                .find(|&s| s.name == *skill_name)
            {
                skill_id = skill.id;
            } else {
                let (skill_name, skill_icon, _) = get_skill_name_and_icon(
                    &skill_id,
                    &skill_effect_id,
                    skill_name.clone(),
                    &self.skill_timestamp,
                    source_entity.id,
                );
                source_encounter_entity.skills.insert(
                    skill_id,
                    Skill {
                        id: skill_id,
                        name: {
                            if skill_name.is_empty() {
                                skill_id.to_string()
                            } else {
                                skill_name
                            }
                        },
                        icon: skill_icon,
                        summon_sources: skill_summon_sources.clone(),
                        casts: 1,
                        ..Default::default()
                    },
                );
            }
        }

        let skills_ptr: *mut hashbrown::HashMap<u32, Skill> = &mut source_encounter_entity.skills;
        let skill = unsafe { (*skills_ptr).get_mut(&skill_id).unwrap() };
        // let skill = source_encounter_entity.skills.get_mut(&skill_id).unwrap();

        let mut skill_hit = SkillHit {
            damage,
            timestamp: relative_timestamp as i64,
            ..Default::default()
        };

        skill.total_damage += damage;
        if damage > skill.max_damage {
            skill.max_damage = damage;
        }
        skill.last_timestamp = timestamp;

        source_encounter_entity.damage_stats.damage_dealt += damage;

        let is_hyper_awakening = is_hyper_awakening_skill(skill.id);
        if is_hyper_awakening {
            source_encounter_entity.damage_stats.hyper_awakening_damage += damage;
        }

        target_encounter_entity.damage_stats.damage_taken += damage;

        source_encounter_entity.skill_stats.hits += 1;
        skill.hits += 1;

        if hit_flag == HitFlag::Critical || hit_flag == HitFlag::DamageOverTimeCritical {
            source_encounter_entity.skill_stats.crits += 1;
            source_encounter_entity.damage_stats.crit_damage += damage;
            skill.crits += 1;
            skill.crit_damage += damage;
            skill_hit.crit = true;
        }
        if hit_option == HitOption::BackAttack {
            source_encounter_entity.skill_stats.back_attacks += 1;
            source_encounter_entity.damage_stats.back_attack_damage += damage;
            skill.back_attacks += 1;
            skill.back_attack_damage += damage;
            skill_hit.back_attack = true;
        }
        if hit_option == HitOption::FrontalAttack {
            source_encounter_entity.skill_stats.front_attacks += 1;
            source_encounter_entity.damage_stats.front_attack_damage += damage;
            skill.front_attacks += 1;
            skill.front_attack_damage += damage;
            skill_hit.front_attack = true;
        }

        let damage_stats = &mut self.encounter.encounter_damage_stats;

        if source_encounter_entity.entity_type == EntityType::Player {

            Self::update_stats_for_player_source(
                timestamp,
                damage,
                is_hyper_awakening,
                &self.custom_id_map,
                &mut self.damage_log,
                skill,
                &mut skill_hit,
                se_on_source,
                vec![],
                se_on_target,
                vec![],
                source_encounter_entity,
                damage_stats
            )
        }

        if target_entity.entity_type == EntityType::Player {
            self.encounter.encounter_damage_stats.total_damage_taken += damage;
            self.encounter.encounter_damage_stats.top_damage_taken = max(
                self.encounter.encounter_damage_stats.top_damage_taken,
                target_encounter_entity.damage_stats.damage_taken,
            );
        }

        // update current_boss
        else if target_encounter_entity.entity_type == EntityType::Boss {
            self.update_boss_hp_log(relative_timestamp, target_encounter_entity);
        }

        if skill_id > 0 {
            self.on_hit(
                source_encounter_entity.id,
                source_entity.id,
                skill_id,
                skill_hit,
                skill_summon_sources,
            );
        }

        result
    }

    fn update_boss_hp_log(
        &mut self,
        relative_timestamp: i32,
        target_encounter_entity: &mut EncounterEntity) {
        self.encounter
            .current_boss_name
            .clone_from(&target_encounter_entity.name);
        target_encounter_entity.id = target_encounter_entity.id;
        target_encounter_entity.npc_id = target_encounter_entity.npc_id;

        let log = self
            .boss_hp_log
            .entry(target_encounter_entity.name.clone())
            .or_default();

        let current_hp = if target_encounter_entity.current_hp >= 0 {
            target_encounter_entity.current_hp + target_encounter_entity.current_shield as i64
        } else {
            0
        };
        let hp_percent = if target_encounter_entity.max_hp != 0 {
            current_hp as f32 / target_encounter_entity.max_hp as f32
        } else {
            0.0
        };

        let relative_timestamp_s = relative_timestamp / 1000;

        if log.is_empty() || log.last().unwrap().time != relative_timestamp_s {
            log.push(BossHpLog::new(relative_timestamp_s, current_hp, hp_percent));
        } else {
            let last = log.last_mut().unwrap();
            last.hp = current_hp;
            last.p = hp_percent;
        }
    }

    pub fn update_stats_for_player_source(
        timestamp: i64,
        damage: i64,
        is_hyper_awakening: bool,
        custom_id_map: &HashMap<u32, u32>,
        damage_log: &mut HashMap<String, Vec<(i64, i64)>>,
        skill: &mut Skill,
        skill_hit: &mut SkillHit,
        se_on_source: Vec<StatusEffectDetails>,
        se_on_source_ids: Vec<u32>,
        se_on_target: Vec<StatusEffectDetails>,
        se_on_target_ids: Vec<u32>,
        source_encounter_entity: &mut EncounterEntity,
        damage_stats: &mut EncounterDamageStats) {
        damage_stats.total_damage_dealt += damage;
        damage_stats.top_damage_dealt = max(
            damage_stats.top_damage_dealt,
            source_encounter_entity.damage_stats.damage_dealt,
        );

        damage_log
            .entry(source_encounter_entity.name.clone())
            .or_default()
            .push((timestamp, damage));

        let mut is_buffed_by_support = false;
        let mut is_buffed_by_identity = false;
        let mut is_debuffed_by_support = false;
        let mut is_buffed_by_hat = false;

        for buff_id in se_on_source_ids.iter() {
            if !damage_stats.unknown_buffs.contains(buff_id)
                && !damage_stats.buffs.contains_key(buff_id)
            {
                let mut source_id: Option<u32> = None;
                let original_buff_id = if let Some(deref_id) = custom_id_map.get(buff_id) {
                    source_id = Some(get_skill_id(*buff_id));
                    *deref_id
                } else {
                    *buff_id
                };

                if let Some(status_effect) = get_status_effect_data(original_buff_id, source_id)
                {
                    damage_stats.buffs.insert(*buff_id, status_effect);
                } else {
                    damage_stats.unknown_buffs.insert(*buff_id);
                }
            }

            if !is_buffed_by_support && !is_hat_buff(buff_id) {
                if let Some(buff) = damage_stats.buffs.get(buff_id) {
                    if let Some(skill) = buff.source.skill.as_ref() {
                        let skill_class: Class = skill.class_id.into();

                        is_buffed_by_support = skill_class.is_support()
                            && buff.buff_type & StatusEffectBuffTypeFlags::DMG.bits() != 0
                            && buff.target == StatusEffectTarget::PARTY
                            && (buff.buff_category == "classskill"
                                || buff.buff_category == "arkpassive");
                    }
                }
            }
            if !is_buffed_by_identity {
                if let Some(buff) = damage_stats.buffs.get(buff_id) {
                    if let Some(skill) = buff.source.skill.as_ref() {
                        let skill_class: Class = skill.class_id.into();

                        is_buffed_by_identity = skill_class.is_support()
                            && buff.buff_type & StatusEffectBuffTypeFlags::DMG.bits() != 0
                            && buff.target == StatusEffectTarget::PARTY
                            && buff.buff_category == "identity";
                    }
                }
            }

            if !is_buffed_by_hat && is_hat_buff(buff_id) {
                is_buffed_by_hat = true;
            }
        }
        
        for debuff_id in se_on_target_ids.iter() {
            if !damage_stats.unknown_buffs.contains(debuff_id)
                && !damage_stats.debuffs.contains_key(debuff_id)
            {
                let mut source_id: Option<u32> = None;
                let original_debuff_id =
                    if let Some(deref_id) = custom_id_map.get(debuff_id) {
                        source_id = Some(get_skill_id(*debuff_id));
                        *deref_id
                    } else {
                        *debuff_id
                    };

                if let Some(status_effect) = get_status_effect_data(original_debuff_id, source_id)
                {
                    damage_stats.debuffs.insert(*debuff_id, status_effect);
                } else {
                    damage_stats.unknown_buffs.insert(*debuff_id);
                }
            }
            if !is_debuffed_by_support {
                if let Some(debuff) = damage_stats.debuffs.get(debuff_id)
                {
                    if let Some(skill) = debuff.source.skill.as_ref() {
                        let skill_class: Class = skill.class_id.into();

                        is_debuffed_by_support = skill_class.is_support()
                            && debuff.buff_type & StatusEffectBuffTypeFlags::DMG.bits() != 0
                            && debuff.target == StatusEffectTarget::PARTY;
                    }
                }
            }
        }

        if is_buffed_by_support && !is_hyper_awakening {
            skill.buffed_by_support += damage;
            source_encounter_entity.damage_stats.buffed_by_support += damage;
        }

        if is_buffed_by_identity && !is_hyper_awakening {
            skill.buffed_by_identity += damage;
            source_encounter_entity.damage_stats.buffed_by_identity += damage;
        }

        if is_debuffed_by_support && !is_hyper_awakening {
            skill.debuffed_by_support += damage;
            source_encounter_entity.damage_stats.debuffed_by_support += damage;
        }

        if is_buffed_by_hat {
            skill.buffed_by_hat += damage;
            source_encounter_entity.damage_stats.buffed_by_hat += damage;
        }

        let stabilized_status_active =
            (source_encounter_entity.current_hp as f64 / source_encounter_entity.max_hp as f64) > 0.65;
        let mut filtered_se_on_source_ids: Vec<u32> = vec![];

        for buff_id in se_on_source_ids.iter() {
            if is_hyper_awakening && !is_hat_buff(buff_id) {
                continue;
            }

            if let Some(buff) = damage_stats.buffs.get(buff_id) {
                if !stabilized_status_active && buff.source.name.contains("Stabilized Status") {
                    continue;
                }
            }

            filtered_se_on_source_ids.push(*buff_id);

            skill
                .buffed_by
                .entry(*buff_id)
                .and_modify(|e| *e += damage)
                .or_insert(damage);
            source_encounter_entity
                .damage_stats
                .buffed_by
                .entry(*buff_id)
                .and_modify(|e| *e += damage)
                .or_insert(damage);
        }

        for debuff_id in se_on_target_ids.iter() {
            if is_hyper_awakening {
                break;
            }

            skill
                .debuffed_by
                .entry(*debuff_id)
                .and_modify(|e| *e += damage)
                .or_insert(damage);
            source_encounter_entity
                .damage_stats
                .debuffed_by
                .entry(*debuff_id)
                .and_modify(|e| *e += damage)
                .or_insert(damage);
        }

        skill_hit.buffed_by = filtered_se_on_source_ids;
        if !is_hyper_awakening {
            skill_hit.debuffed_by = se_on_target_ids;
        }
    }
}