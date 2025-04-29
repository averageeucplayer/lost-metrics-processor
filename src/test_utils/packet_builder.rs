use lost_metrics_core::models::*;
use lost_metrics_sniffer_stub::packets::{common::SkillMoveOptionData, definitions::*, opcodes::Pkt, structures::*};

use super::*;

pub struct PacketBuilder;

impl PacketBuilder {
    pub fn counterattack(source_id: u64) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::CounterAttackNotify;
        let data = PKTCounterAttackNotify {
            source_id,
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn death(target_id: u64) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::DeathNotify;
        let data = PKTDeathNotify {
            target_id,
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn local_player(template: &PlayerTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::InitPC;
        let data = PKTInitPC {
            player_id: template.id,
            name: template.name.to_string(),
            character_id: template.character_id,
            class_id: template.class_id,
            gear_level: template.gear_level,
            stat_pairs: template.stat_pairs.to_vec(),
            status_effect_datas: template.status_effect_datas.to_vec(),
            
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn new_player(template: &PlayerTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::NewPC;
        let data = PKTNewPC {
            pc_struct:  PKTNewPCInner {
                player_id: template.id,
                name: template.name.to_string(),
                character_id: template.character_id,
                class_id: template.class_id,
                max_item_level: 1700.0,
                stat_pairs: template.stat_pairs.to_vec(),
                status_effect_datas: template.status_effect_datas.to_vec(),
                equip_item_datas: vec![]
            }
            
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn zone_member_load(zone_id: u32, zone_level: u32) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::ZoneMemberLoadStatusNotify;
        let data = PKTZoneMemberLoadStatusNotify {
            zone_id,
            zone_level,
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn skill_start(
        source_id: u64,
        skill_id: u32,
        tripod_index: Option<lost_metrics_sniffer_stub::packets::definitions::TripodIndex>,
        tripod_level: Option<lost_metrics_sniffer_stub::packets::definitions::TripodLevel>
    ) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::SkillStartNotify;
        let data = PKTSkillStartNotify {
            source_id,
            skill_id,
            skill_option_data: PKTSkillStartNotifyInner {
                tripod_index,
                tripod_level,
            }
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn skill_cast(source_id: u64, skill_id: u32) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::SkillCastNotify;
        let data = PKTSkillCastNotify {
            skill_id,
            source_id
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn skill_damage_abnormal(
        source_id: u64,
        target_id: u64,
        skill_id: u32,
        damage: i64,
        shield_damage: Option<i64>,
        hit_option: HitOption,
        hit_flag: HitFlag,
        cur_hp: i64,
        max_hp: i64,
        down_time: Option<f32>,
        stand_up_time: Option<f32>,
        move_time: Option<f32>
    ) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::SkillDamageAbnormalMoveNotify;
        let data = PKTSkillDamageAbnormalMoveNotify {
            source_id,
            skill_damage_abnormal_move_events: vec![
                PKTSkillDamageAbnormalMoveNotifyInner {
                    skill_damage_event: SkillDamageEvent { 
                        target_id,
                        damage,
                        modifier: to_modifier(hit_option, hit_flag),
                        cur_hp,
                        max_hp,
                        damage_attr: None,
                        damage_type: 0,
                        sub_p_k_t_skill_damage_abnormal_move_notify_4_2_9: SkillDamageEventInner {
                            p64_0: shield_damage,
                        }
                    },
                    skill_move_option_data: SkillMoveOptionData {
                        down_time,
                        stand_up_time,
                        move_time,
                    }
                }
            ],
            skill_id,
            skill_effect_id: 0
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn skill_damage(
        source_id: u64,
        target_id: u64,
        skill_id: u32,
        damage: i64,
        shield_damage: Option<i64>,
        hit_option: HitOption,
        hit_flag: HitFlag,
        cur_hp: i64,
        max_hp: i64,
    ) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::SkillDamageNotify;
        let data = PKTSkillDamageNotify {
            source_id,
            skill_damage_events: vec![
                SkillDamageEvent { 
                    target_id,
                    damage,
                    modifier: to_modifier(hit_option, hit_flag),
                    cur_hp,
                    max_hp,
                    damage_attr: None,
                    damage_type: 0,
                    sub_p_k_t_skill_damage_abnormal_move_notify_4_2_9: SkillDamageEventInner {
                        p64_0: shield_damage,
                    }
                }
            ],
            skill_id,
            skill_effect_id: None
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn identity_change(player_id: u64) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::IdentityGaugeChangeNotify;
        let data = PKTIdentityGaugeChangeNotify {
            player_id,
            identity_gauge1: 1,
            identity_gauge2: 1,
            identity_gauge3: 1
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn new_npc(template: &NpcTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::NewNpc;
        let data = PKTNewNpc {
            npc_struct:  NpcStruct {
                type_id: 1,
                object_id: 1,
                level: 60,
                balance_level: None,
                stat_pairs: vec![],
                status_effect_datas: vec![],
            }
            
        };

        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn new_npc_summon(template: &NpcTemplate)-> (Pkt, Vec<u8>) {
        let opcode = Pkt::NewNpcSummon;
        let data = PKTNewNpcSummon {
            owner_id: 1,
            npc_struct: NpcStruct {
                object_id: 1,
                type_id: 1,
                level: 60,
                balance_level: None,
                stat_pairs: vec![],
                status_effect_datas: vec![]
            },
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn new_projectile(template: &ProjectileTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::NewProjectile;
        let data = PKTNewProjectile {
            projectile_info: PKTNewProjectileInner { 
                projectile_id: template.projectile_id,
                owner_id: template.owner_id,
                skill_id: template.skill_id,
                skill_effect: template.skill_effect
            }
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn new_trap(template: &TrapTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::NewTrap;
        let data = PKTNewTrap {
            trap_struct: PKTNewTrapInner {
                object_id: template.object_id,
                owner_id: template.owner_id,
                skill_id: template.skill_id,
                skill_effect: template.skill_effect
            }
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn raid_begin(raid_id: u32) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::RaidBegin;
        let data = PKTRaidBegin {
            raid_id,
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn init_env(player_id: u64) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::InitEnv;
        let data = PKTInitEnv {
            player_id,
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn trigger_start(signal: u32)-> (Pkt, Vec<u8>) {
        let opcode = Pkt::TriggerStartNotify;
        let data = PKTTriggerStartNotify {
            signal,
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn raid_result() -> (Pkt, Vec<u8>) {
        let opcode = Pkt::RaidResult;
        let data = vec![];

        (opcode, data)
    }

    pub fn raid_boss_kill() -> (Pkt, Vec<u8>) {
        let opcode = Pkt::RaidBossKillNotify;
        let data = vec![];

        (opcode, data)
    }

    pub fn remove_object(object_id: u64) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::RemoveObject;
        let data = PKTRemoveObject {
            unpublished_objects: vec![
                PKTRemoveObjectInner {
                    object_id,
            }],
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn zone_object_unpublish(object_id: u64) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::ZoneObjectUnpublishNotify;
        let data = PKTZoneObjectUnpublishNotify {
            object_id: 1
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn status_effect_add(object_id: u64, template: StatusEffectTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::StatusEffectAddNotify;
        let data = PKTStatusEffectAddNotify {
            object_id,
            status_effect_data: StatusEffectData {
                source_id: template.source_id,
                status_effect_id: template.status_effect_id,
                status_effect_instance_id: template.status_effect_instance_id,
                value: StatusEffectDataValue { bytearray_0: template.value },
                total_time: template.total_time,
                stack_count: template.stack_count,
                end_tick: template.end_tick
            }
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn party_status_effect_add(character_id: u64, template: StatusEffectTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::PartyStatusEffectAddNotify;
        let data = PKTPartyStatusEffectAddNotify {
            character_id,
            status_effect_datas: vec![StatusEffectData {
                source_id: template.source_id,
                status_effect_id: template.status_effect_id,
                status_effect_instance_id: template.status_effect_instance_id,
                value: StatusEffectDataValue { bytearray_0: template.value },
                total_time: template.total_time,
                stack_count: template.stack_count,
                end_tick: template.end_tick,
            }]
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn party_status_effect_remove(character_id: u64, instance_ids: Vec<u32>, reason: u8) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::PartyStatusEffectRemoveNotify;
        let data = PKTPartyStatusEffectRemoveNotify {
            character_id,
            status_effect_instance_ids: instance_ids,
            reason
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn status_effect_remove(object_id: u64, reason: u8, template: &StatusEffectTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::StatusEffectRemoveNotify;
        let data = PKTStatusEffectRemoveNotify {
            object_id: object_id,
            status_effect_instance_ids: vec![template.status_effect_instance_id],
            reason
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn party_status_effect_result(
        character_id: u64,
        party_instance_id: u32,
        raid_instance_id: u32) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::PartyStatusEffectResultNotify;
        let data = PKTPartyStatusEffectResultNotify {
            character_id,
            party_instance_id,
            raid_instance_id
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn status_effect_sync(
        character_id: u64,
        object_id: u64,
        value: u64,
        status_effect_instance_id: u32
    ) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::StatusEffectSyncDataNotify;
        let data = PKTStatusEffectSyncDataNotify {
            character_id,
            object_id,
            value,
            status_effect_instance_id
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn troop_member_update(
        character_id: u64,
        cur_hp: i64,
        max_hp: i64,
        template: &StatusEffectTemplate
    ) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::TroopMemberUpdateMinNotify;
        let data = PKTTroopMemberUpdateMinNotify {
            character_id,
            cur_hp,
            max_hp,
            status_effect_datas: vec![
                StatusEffectData { 
                    source_id: template.source_id,
                    status_effect_id: template.status_effect_id,
                    status_effect_instance_id: template.status_effect_instance_id,
                    value: StatusEffectDataValue { bytearray_0: template.value.clone() },
                    total_time: template.total_time,
                    stack_count: template.stack_count,
                    end_tick: template.end_tick
                }
            ]
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn party_info(template: &PartyTemplate) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::PartyInfo;
        let data = PKTPartyInfo {
            party_instance_id: template.party_instance_id,
            raid_instance_id: template.raid_instance_id,
            party_member_datas: template.members.iter().map(|pr| 
                PKTPartyInfoInner {
                    character_id: pr.character_id,
                    class_id: pr.class_id,
                    gear_level: pr.gear_level,
                    name: pr.name.to_string()
                }
            ).collect()
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn party_leave(name: String, party_instance_id: u32) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::PartyLeaveResult;
        let data = PKTPartyLeaveResult {
            name,
            party_instance_id
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }

    pub fn trigger_boss_battle_status() -> (Pkt, Vec<u8>) {
        let opcode = Pkt::TriggerBossBattleStatus;
        let data = vec![];

        (opcode, data)
    }

    pub fn new_transit(channel_id: u32) -> (Pkt, Vec<u8>) {
        let opcode = Pkt::NewTransit;
        let data = PKTNewTransit {
            channel_id
        };
        let data = data.encode().unwrap();

        (opcode, data)
    }
}