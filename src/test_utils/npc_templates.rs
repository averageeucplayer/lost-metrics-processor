use lost_metrics_sniffer_stub::packets::structures::{StatPair, StatusEffectData};

pub struct NpcTemplate {    
    pub object_id: u64,
    pub name: &'static str,
    pub owner_id: u64,
    pub type_id: u32,
    pub level: u16,
    pub balance_level: Option<u16>,
    pub stat_pairs: [StatPair; 2],
    pub status_effect_datas: [StatusEffectData; 0]
}

pub const NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER: NpcTemplate = NpcTemplate {
    name: "Thaemine the Lightqueller",
    object_id: 1000,
    owner_id: 0,
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

pub const NPC_TEMPLATE_TABOO_KIN: NpcTemplate = NpcTemplate {
    name: "Taboo Kin",
    object_id: 1001,
    owner_id: 0,
    type_id: 480480,
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

