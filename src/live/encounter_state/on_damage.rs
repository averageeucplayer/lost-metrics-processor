use lost_metrics_core::models::*;
use lost_metrics_misc::*;
use std::cmp::max;
use std::default::Default;
use std::sync::Arc;

use crate::live::abstractions::EventEmitter;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {
        
    #[allow(clippy::too_many_arguments)]
    pub fn on_damage<EE: EventEmitter>(
        &mut self,
        dmg_src_entity: &Entity,
        proj_entity: &Entity,
        dmg_target_entity: &Entity,
        damage_data: DamageData,
        se_on_source: Vec<StatusEffectDetails>,
        se_on_target: Vec<StatusEffectDetails>,
        _target_count: i32,
        timestamp: i64,
        event_emitter: Arc<EE>
    ) {
        let hit_flag: HitFlag = (damage_data.modifier & 0xf).into();
        let hit_option: HitOption = ((damage_data.modifier >> 4) & 0x7).into();
        
        if hit_flag == HitFlag::Invincible {
            return;
        }
        if hit_flag == HitFlag::DamageShare
            && damage_data.skill_id == 0
            && damage_data.skill_effect_id == 0
        {
            return;
        }

        let mut skill_effect_id = damage_data.skill_effect_id;
        if proj_entity.entity_type == EntityType::Projectile
            && is_battle_item(&proj_entity.skill_effect_id, "attack")
        {
            skill_effect_id = proj_entity.skill_effect_id;
        }

        let mut source_entity = self
            .encounter
            .entities
            .entry(dmg_src_entity.name.clone())
            .or_insert_with(|| encounter_entity_from_entity(dmg_src_entity))
            .to_owned();

        let mut target_entity = self
            .encounter
            .entities
            .entry(dmg_target_entity.name.clone())
            .or_insert_with(|| {
                let mut target_entity = encounter_entity_from_entity(dmg_target_entity);
                target_entity.current_hp = damage_data.target_current_hp;
                target_entity.max_hp = damage_data.target_max_hp;
                target_entity
            })
            .to_owned();

        // if boss only damage is enabled
        // check if target is boss and not player
        // check if target is player and source is boss
        if self.boss_only_damage
            && ((target_entity.entity_type != EntityType::Boss
                && target_entity.entity_type != EntityType::Player)
                || (target_entity.entity_type == EntityType::Player
                    && source_entity.entity_type != EntityType::Boss))
        {
            return;
        }

        if self.encounter.fight_start == 0 {
            self.encounter.fight_start = timestamp;
            self.skill_tracker.fight_start = timestamp;
            if source_entity.entity_type == EntityType::Player && damage_data.skill_id > 0 {
                self.skill_tracker.new_cast(
                    source_entity.id,
                    damage_data.skill_id,
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
            event_emitter
                .emit("raid-start", timestamp)
                .expect("failed to emit raid-start");
            // self.window
            //     .emit("raid-start", timestamp)
            //     .expect("failed to emit raid-start");
        }

        self.encounter.last_combat_packet = timestamp;

        source_entity.id = dmg_src_entity.id;

        if target_entity.id == dmg_target_entity.id {
            target_entity.current_hp = damage_data.target_current_hp;
            target_entity.max_hp = damage_data.target_max_hp;
        }

        let mut damage = damage_data.damage;
        if target_entity.entity_type != EntityType::Player && damage_data.target_current_hp < 0 {
            damage += damage_data.target_current_hp;
        }

        let mut skill_id = if damage_data.skill_id != 0 {
            damage_data.skill_id
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
                &self.skill_tracker.skill_timestamp,
                source_entity.id,
            );
        }
        let relative_timestamp = (timestamp - self.encounter.fight_start) as i32;

        if !source_entity.skills.contains_key(&skill_id) {
            if let Some(skill) = source_entity
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
                    &self.skill_tracker.skill_timestamp,
                    source_entity.id,
                );
                source_entity.skills.insert(
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

        let skill = source_entity.skills.get_mut(&skill_id).unwrap();

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

        source_entity.damage_stats.damage_dealt += damage;

        let is_hyper_awakening = is_hyper_awakening_skill(skill.id);
        if is_hyper_awakening {
            source_entity.damage_stats.hyper_awakening_damage += damage;
        }

        target_entity.damage_stats.damage_taken += damage;

        source_entity.skill_stats.hits += 1;
        skill.hits += 1;

        if hit_flag == HitFlag::Critical || hit_flag == HitFlag::DamageOverTimeCritical {
            source_entity.skill_stats.crits += 1;
            source_entity.damage_stats.crit_damage += damage;
            skill.crits += 1;
            skill.crit_damage += damage;
            skill_hit.crit = true;
        }
        if hit_option == HitOption::BackAttack {
            source_entity.skill_stats.back_attacks += 1;
            source_entity.damage_stats.back_attack_damage += damage;
            skill.back_attacks += 1;
            skill.back_attack_damage += damage;
            skill_hit.back_attack = true;
        }
        if hit_option == HitOption::FrontalAttack {
            source_entity.skill_stats.front_attacks += 1;
            source_entity.damage_stats.front_attack_damage += damage;
            skill.front_attacks += 1;
            skill.front_attack_damage += damage;
            skill_hit.front_attack = true;
        }

        if source_entity.entity_type == EntityType::Player {
            self.encounter.encounter_damage_stats.total_damage_dealt += damage;
            self.encounter.encounter_damage_stats.top_damage_dealt = max(
                self.encounter.encounter_damage_stats.top_damage_dealt,
                source_entity.damage_stats.damage_dealt,
            );

            self.damage_log
                .entry(source_entity.name.clone())
                .or_default()
                .push((timestamp, damage));

            let mut is_buffed_by_support = false;
            let mut is_buffed_by_identity = false;
            let mut is_debuffed_by_support = false;
            let mut is_buffed_by_hat = false;
            let se_on_source_ids = se_on_source
                .iter()
                .map(|se| map_status_effect(se, &mut self.custom_id_map))
                .collect::<Vec<_>>();
            for buff_id in se_on_source_ids.iter() {
                if !self
                    .encounter
                    .encounter_damage_stats
                    .unknown_buffs
                    .contains(buff_id)
                    && !self
                        .encounter
                        .encounter_damage_stats
                        .buffs
                        .contains_key(buff_id)
                {
                    let mut source_id: Option<u32> = None;
                    let original_buff_id = if let Some(deref_id) = self.custom_id_map.get(buff_id) {
                        source_id = Some(get_skill_id(*buff_id));
                        *deref_id
                    } else {
                        *buff_id
                    };

                    if let Some(status_effect) = get_status_effect_data(original_buff_id, source_id)
                    {
                        self.encounter
                            .encounter_damage_stats
                            .buffs
                            .insert(*buff_id, status_effect);
                    } else {
                        self.encounter
                            .encounter_damage_stats
                            .unknown_buffs
                            .insert(*buff_id);
                    }
                }
                if !is_buffed_by_support && !is_hat_buff(buff_id) {
                    if let Some(buff) = self.encounter.encounter_damage_stats.buffs.get(buff_id) {
                        if let Some(skill) = buff.source.skill.as_ref() {
                            is_buffed_by_support = is_support_class_id(skill.class_id)
                                && buff.buff_type & StatusEffectBuffTypeFlags::DMG.bits() != 0
                                && buff.target == StatusEffectTarget::PARTY
                                && (buff.buff_category == "classskill"
                                    || buff.buff_category == "arkpassive");
                        }
                    }
                }
                if !is_buffed_by_identity {
                    if let Some(buff) = self.encounter.encounter_damage_stats.buffs.get(buff_id) {
                        if let Some(skill) = buff.source.skill.as_ref() {
                            is_buffed_by_identity = is_support_class_id(skill.class_id)
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
            let se_on_target_ids = se_on_target
                .iter()
                .map(|se| map_status_effect(se, &mut self.custom_id_map))
                .collect::<Vec<_>>();
            for debuff_id in se_on_target_ids.iter() {
                if !self
                    .encounter
                    .encounter_damage_stats
                    .unknown_buffs
                    .contains(debuff_id)
                    && !self
                        .encounter
                        .encounter_damage_stats
                        .debuffs
                        .contains_key(debuff_id)
                {
                    let mut source_id: Option<u32> = None;
                    let original_debuff_id =
                        if let Some(deref_id) = self.custom_id_map.get(debuff_id) {
                            source_id = Some(get_skill_id(*debuff_id));
                            *deref_id
                        } else {
                            *debuff_id
                        };

                    if let Some(status_effect) =
                        get_status_effect_data(original_debuff_id, source_id)
                    {
                        self.encounter
                            .encounter_damage_stats
                            .debuffs
                            .insert(*debuff_id, status_effect);
                    } else {
                        self.encounter
                            .encounter_damage_stats
                            .unknown_buffs
                            .insert(*debuff_id);
                    }
                }
                if !is_debuffed_by_support {
                    if let Some(debuff) =
                        self.encounter.encounter_damage_stats.debuffs.get(debuff_id)
                    {
                        if let Some(skill) = debuff.source.skill.as_ref() {
                            is_debuffed_by_support = is_support_class_id(skill.class_id)
                                && debuff.buff_type & StatusEffectBuffTypeFlags::DMG.bits() != 0
                                && debuff.target == StatusEffectTarget::PARTY;
                        }
                    }
                }
            }

            if is_buffed_by_support && !is_hyper_awakening {
                skill.buffed_by_support += damage;
                source_entity.damage_stats.buffed_by_support += damage;
            }
            if is_buffed_by_identity && !is_hyper_awakening {
                skill.buffed_by_identity += damage;
                source_entity.damage_stats.buffed_by_identity += damage;
            }
            if is_debuffed_by_support && !is_hyper_awakening {
                skill.debuffed_by_support += damage;
                source_entity.damage_stats.debuffed_by_support += damage;
            }
            if is_buffed_by_hat {
                skill.buffed_by_hat += damage;
                source_entity.damage_stats.buffed_by_hat += damage;
            }

            let stabilized_status_active =
                (source_entity.current_hp as f64 / source_entity.max_hp as f64) > 0.65;
            let mut filtered_se_on_source_ids: Vec<u32> = vec![];

            for buff_id in se_on_source_ids.iter() {
                if is_hyper_awakening && !is_hat_buff(buff_id) {
                    continue;
                }

                if let Some(buff) = self.encounter.encounter_damage_stats.buffs.get(buff_id) {
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
                source_entity
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
                source_entity
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

        if target_entity.entity_type == EntityType::Player {
            self.encounter.encounter_damage_stats.total_damage_taken += damage;
            self.encounter.encounter_damage_stats.top_damage_taken = max(
                self.encounter.encounter_damage_stats.top_damage_taken,
                target_entity.damage_stats.damage_taken,
            );
        }
        // update current_boss
        else if target_entity.entity_type == EntityType::Boss {
            self.encounter
                .current_boss_name
                .clone_from(&target_entity.name);
            target_entity.id = dmg_target_entity.id;
            target_entity.npc_id = dmg_target_entity.npc_id;

            let log = self
                .boss_hp_log
                .entry(target_entity.name.clone())
                .or_default();

            let current_hp = if target_entity.current_hp >= 0 {
                target_entity.current_hp + target_entity.current_shield as i64
            } else {
                0
            };
            let hp_percent = if target_entity.max_hp != 0 {
                current_hp as f32 / target_entity.max_hp as f32
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

        if skill_id > 0 {
            self.skill_tracker.on_hit(
                source_entity.id,
                proj_entity.id,
                skill_id,
                skill_hit,
                skill_summon_sources,
            );
        }

        self.encounter
            .entities
            .insert(source_entity.name.clone(), source_entity);
        self.encounter
            .entities
            .insert(target_entity.name.clone(), target_entity);
    }
}