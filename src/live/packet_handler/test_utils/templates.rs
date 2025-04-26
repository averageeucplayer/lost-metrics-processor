use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::*, stats_api::MockStatsApi};
use crate::live::test_utils::*;
use lost_metrics_data::{NPC_DATA, SKILL_BUFF_DATA};
use lost_metrics_sniffer_stub::packets::{common::SkillMoveOptionData, definitions::{PKTNewPC, PKTNewPCInner}, structures::{NpcStruct, SkillDamageEvent, StatPair, StatusEffectData}};
use lost_metrics_store::encounter_service;
use serde::Serialize;
use std::fmt::Debug;
use mockall::*;

pub struct PlayerTemplate {
    pub id: u64,
    pub character_id: u64,
    pub class_id: u32,
    pub gear_level: f32,
    pub name: &'static str,
    pub stat_pairs: [StatPair; 2],
    pub status_effect_datas: [StatusEffectData; 2]
}

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
    pub character_id: u64,
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
    character_id: 0,
    source_id: 0,
    status_effect_id: 602000088,
    status_effect_instance_id: 9001,
    value: None,
    total_time: 1000.0,
    stack_count: 0,
    end_tick: 04
};

pub const STATUS_EFFECT_TEMPLATE_SHIELD: StatusEffectTemplate = StatusEffectTemplate {
    character_id: 0,
    source_id: 0,
    status_effect_id: 700006103,
    status_effect_instance_id: 9002,
    value: None,
    total_time: 1000.0,
    stack_count: 0,
    end_tick: 04
};

pub const STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF: StatusEffectTemplate = StatusEffectTemplate {
    character_id: 2,
    source_id: 2,
    status_effect_id: 211606,
    status_effect_instance_id: 20002,
    value: None,
    total_time: 8.0,
    stack_count: 0,
    end_tick: 04
};

pub const TRAP_TEMPLATE_BARD_STIGMA: TrapTemplate = TrapTemplate {
    object_id: 3000,
    owner_id: 0,
    skill_id: BardSkills::Stigma as u32,
    skill_effect: 0
};

pub const PROJECTILE_TEMPLATE_SORCERESS_DESTRUCTION: ProjectileTemplate = ProjectileTemplate {
    projectile_id: 2000,
    owner_id: 0,
    skill_id: SorceressSkills::Destruction as u32,
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

pub const PLAYER_TEMPLATE_SORCERESS: PlayerTemplate = PlayerTemplate {
    id: 1,
    character_id: 1,
    class_id: Class::Sorceress as u32,
    gear_level: 1700.0,
    name: "TestPlayerSorceress",
    stat_pairs: [
        StatPair {
            stat_type: 1,
            value: 100000
        },
        StatPair {
            stat_type: 27,
            value: 100000
        },
    ],
    status_effect_datas: [
        StatusEffectData {
            source_id: 1,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 0,
            status_effect_instance_id: 1001,
            total_time: 10000.0,
            value: Some(vec![])
        },
        StatusEffectData {
            source_id: 1,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 0,
            status_effect_instance_id: 1002,
            total_time: 10000.0,
            value: Some(vec![])
        }
    ]
};

pub const PLAYER_TEMPLATE_BARD: PlayerTemplate = PlayerTemplate {
    id: 2,
    character_id: 2,
    class_id: Class::Bard as u32,
    gear_level: 1700.0,
    name: "TestPlayerBard",
    stat_pairs: [
        StatPair {
            stat_type: 1,
            value: 100000
        },
        StatPair {
            stat_type: 27,
            value: 100000
        },
    ],
    status_effect_datas: [
        StatusEffectData {
            source_id: 2,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 20004, // Vitality Training
            status_effect_instance_id: 2001,
            total_time: 10000.0,
            value: Some(vec![])
        },
        StatusEffectData {
            source_id: 2,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21304, // Improved Swiftness
            status_effect_instance_id: 2002,
            total_time: 10000.0,
            value: Some(vec![])
        }
    ]
};

pub const PLAYER_TEMPLATE_BERSERKER: PlayerTemplate = PlayerTemplate {
    id: 3,
    character_id: 3,
    class_id: Class::Berserker as u32,
    gear_level: 1700.0,
    name: "TestPlayerBerserker",
    stat_pairs: [
        StatPair {
            stat_type: 1,
            value: 100000
        },
        StatPair {
            stat_type: 27,
            value: 100000
        },
    ],
    status_effect_datas: [
        StatusEffectData {
            source_id: 3,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 20004, // Vitality Training
            status_effect_instance_id: 3001,
            total_time: 10000.0,
            value: Some(vec![])
        },
        StatusEffectData {
            source_id: 3,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21004, //  Improved Crit
            status_effect_instance_id: 3002,
            total_time: 10000.0,
            value: Some(vec![])
        }
    ]
};

pub const PLAYER_TEMPLATE_SOULEATER: PlayerTemplate = PlayerTemplate {
    id: 4,
    character_id: 4,
    class_id: Class::Souleater as u32,
    gear_level: 1700.0,
    name: "TestPlayerSouleater",
    stat_pairs: [
        StatPair {
            stat_type: 1,
            value: 100000
        },
        StatPair {
            stat_type: 27,
            value: 100000
        },
    ],
    status_effect_datas: [
        StatusEffectData {
            source_id: 4,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 20004, // Vitality Training
            status_effect_instance_id: 4001,
            total_time: 10000.0,
            value: Some(vec![])
        },
        StatusEffectData {
            source_id: 4,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21004, //  Improved Crit
            status_effect_instance_id: 4002,
            total_time: 10000.0,
            value: Some(vec![])
        }
    ]
};
