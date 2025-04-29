use lost_metrics_core::models::Class;
use lost_metrics_sniffer_stub::packets::structures::{StatPair, StatusEffectData, StatusEffectDataValue};

pub struct PlayerTemplate {
    pub id: u64,
    pub character_id: u64,
    pub class_id: u32,
    pub gear_level: f32,
    pub name: &'static str,
    pub stat_pairs: [StatPair; 2],
    pub status_effect_datas: [StatusEffectData; 2]
}

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
            status_effect_id: 20004, // Vitality Training
            status_effect_instance_id: 1001,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        },
        StatusEffectData {
            source_id: 1,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21004, //  Improved Crit
            status_effect_instance_id: 1002,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        }
    ]
};

pub const PLAYER_TEMPLATE_BARD: PlayerTemplate = PlayerTemplate {
    id: 2,
    character_id: 2,
    class_id: Class::Bard as u32,
    gear_level: 1700.0,
    name: "Testplayerbard",
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
            value: StatusEffectDataValue { bytearray_0: None }
        },
        StatusEffectData {
            source_id: 2,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21304, // Improved Swiftness
            status_effect_instance_id: 2002,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        }
    ]
};

pub const PLAYER_TEMPLATE_BERSERKER: PlayerTemplate = PlayerTemplate {
    id: 3,
    character_id: 3,
    class_id: Class::Berserker as u32,
    gear_level: 1700.0,
    name: "Testplayerberserker",
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
            value: StatusEffectDataValue { bytearray_0: None }
        },
        StatusEffectData {
            source_id: 3,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21004, //  Improved Crit
            status_effect_instance_id: 3002,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        }
    ]
};

pub const PLAYER_TEMPLATE_SOULEATER: PlayerTemplate = PlayerTemplate {
    id: 4,
    character_id: 4,
    class_id: Class::Souleater as u32,
    gear_level: 1700.0,
    name: "Testplayersouleater",
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
            value: StatusEffectDataValue { bytearray_0: None }
        },
        StatusEffectData {
            source_id: 4,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21004, //  Improved Crit
            status_effect_instance_id: 4002,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        }
    ]
};

pub const PLAYER_TEMPLATE_DEATHBLADE: PlayerTemplate = PlayerTemplate {
    id: 5,
    character_id: 5,
    class_id: Class::Deathblade as u32,
    gear_level: 1700.0,
    name: "Testplayerdeathblade",
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
            source_id: 5,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 20004, // Vitality Training
            status_effect_instance_id: 5001,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        },
        StatusEffectData {
            source_id: 4,
            end_tick: 0,
            stack_count: 0,
            status_effect_id: 21104, //  Improved Specialization
            status_effect_instance_id: 5002,
            total_time: 10000.0,
            value: StatusEffectDataValue { bytearray_0: None }
        }
    ]
};
