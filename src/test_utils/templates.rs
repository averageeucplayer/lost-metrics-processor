use lost_metrics_core::models::*;

use super::PlayerTemplate;

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

pub const PROJECTILE_TEMPLATE_DARK_GRENADE: ProjectileTemplate = ProjectileTemplate {
    projectile_id: 2000,
    owner_id: 0,
    skill_id: 0,
    skill_effect: 32240
};

