use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::*, stats_api::MockStatsApi};
use crate::live::test_utils::*;
use lost_metrics_data::{NPC_DATA, SKILL_BUFF_DATA};
use lost_metrics_sniffer_stub::packets::{common::SkillMoveOptionData, definitions::{PKTNewPC, PKTNewPCInner}, structures::{NpcStruct, SkillDamageEvent, StatPair, StatusEffectData}};
use lost_metrics_store::encounter_service;
use serde::Serialize;
use std::fmt::Debug;
use mockall::*;

use super::PlayerTemplate;


pub struct NpcTemplate {    
    pub object_id: u64,
    pub name: &'static str,
    pub type_id: u32,
    pub level: u16,
    pub balance_level: Option<u16>,
    pub stat_pairs: [StatPair; 2],
    pub status_effect_datas: [StatusEffectData; 0]
}

pub struct PartyTemplate {
    pub party_instance_id: u32,
    pub raid_instance_id: u32,
    pub members: [PlayerTemplate; 4]
}

pub struct StatusEffectTemplate {
    pub source_id: u64,
    pub status_effect_id: u32,
    pub status_effect_instance_id: u32,
    pub value: Option<Vec<u8>>,
    pub total_time: f32,
    pub stack_count: u8,
    pub end_tick: u64
}

pub struct TrapTemplate {
    pub object_id: u64,
    pub owner_id: u64,
    pub skill_id: u32,
    pub skill_effect: u32
}

pub struct ProjectileTemplate {
    pub projectile_id: u64,
    pub owner_id: u64,
    pub skill_id: u32,
    pub skill_effect: u32
}

pub const STATUS_EFFECT_TEMPLATE_FREEZE: StatusEffectTemplate = StatusEffectTemplate {
    source_id: 0,
    status_effect_id: 602000088,
    status_effect_instance_id: 9001,
    value: None,
    total_time: 1000.0,
    stack_count: 0,
    end_tick: 0
};

pub const STATUS_EFFECT_TEMPLATE_BARD_WIND_OF_MUSIC_SHIELD: StatusEffectTemplate = StatusEffectTemplate {
    source_id: 0,
    status_effect_id: 210709,
    status_effect_instance_id: 9002,
    value: None,
    total_time: 1000.0,
    stack_count: 0,
    end_tick: 0
};

pub const STATUS_EFFECT_TEMPLATE_SHIELD: StatusEffectTemplate = StatusEffectTemplate {
    source_id: 0,
    status_effect_id: 700006103,
    status_effect_instance_id: 9002,
    value: None,
    total_time: 1000.0,
    stack_count: 0,
    end_tick: 0
};

pub const STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF: StatusEffectTemplate = StatusEffectTemplate {
    source_id: 2,
    status_effect_id: 211606,
    status_effect_instance_id: 20002,
    value: None,
    total_time: 8.0,
    stack_count: 0,
    end_tick: 0
};

pub const TRAP_TEMPLATE_BARD_STIGMA: TrapTemplate = TrapTemplate {
    object_id: 3000,
    owner_id: 0,
    skill_id: BardSkills::Stigma as u32,
    skill_effect: 0
};

pub const PROJECTILE_TEMPLATE_SORCERESS_EXPLOSION: ProjectileTemplate = ProjectileTemplate {
    projectile_id: 2000,
    owner_id: 0,
    skill_id: SorceressSkills::Explosion as u32,
    skill_effect: 0
};

pub const NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER: NpcTemplate = NpcTemplate {
    name: "Thaemine the Lightqueller",
    object_id: 1000,
    type_id: 480544,
    level: 60,
    balance_level: Some(60),
    stat_pairs: [
        StatPair {
            stat_type: 1,
            value: 1e10 as i64
        },
        StatPair {
            stat_type: 27,
            value: 1e10 as i64
        },
    ],
    status_effect_datas: []
};

