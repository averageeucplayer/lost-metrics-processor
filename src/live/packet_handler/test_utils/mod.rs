mod templates;
mod packet_builder;
mod packet_handler_builder;
pub mod state_builder;

pub use templates::*;
pub use packet_builder::*;
pub use packet_handler_builder::*;
pub use state_builder::*;

use std::{cell::RefCell, rc::Rc, sync::{Arc, RwLock}};
use crate::live::{abstractions::*, encounter_state::EncounterState, flags::MockFlags, packet_handler::*, stats_api::MockStatsApi};
use crate::live::test_utils::*;
use lost_metrics_data::{NPC_DATA, SKILL_BUFF_DATA};
use lost_metrics_sniffer_stub::packets::{common::SkillMoveOptionData, definitions::{PKTNewPC, PKTNewPCInner}, structures::{NpcStruct, SkillDamageEvent, StatPair, StatusEffectData}};
use lost_metrics_store::encounter_service;
use serde::Serialize;
use std::fmt::Debug;
use mockall::*;


pub fn to_modifier(hit_option: HitOption, hit_flag: HitFlag) -> i32 {
    (hit_flag as i32) | ((hit_option as i32) << 4)
}

pub fn get_skill_buff_by_type_and_lt_duration(kind: &str, duration: i32) -> &SkillBuffData {
    SKILL_BUFF_DATA
        .iter()
        .filter(|(id, buff)| buff.buff_type == kind
            && buff.duration < duration)
        .map(|(id, buff)| buff)
        .max_by_key(|buff| buff.duration)
        .unwrap()
}

pub fn get_npc_by_name<'a>(npc_name: &str) -> Option<&'a Npc> {
    NPC_DATA
        .iter()
        .filter(|(id, npc)| 
            npc.hp_bars > 1
            && npc.name.as_ref().filter(|name| *name == npc_name).is_some())
        .max_by_key(|(_, npc)| npc.grade)
        .map(|(_, npc)| npc)
}

pub fn create_npc(object_id: u64, name: &str) -> PKTNewNpc {

    let npc = get_npc_by_name(name).expect("Provide valid npc name");

    PKTNewNpc {
        npc_struct: NpcStruct {
            type_id: npc.id as u32,
            object_id,
            level: 60,
            balance_level: None,
            stat_pairs: vec![],
            status_effect_datas: vec![],
        }   
    }
}

pub fn create_pc(player_id: u64, class_id: u32, character_id: u64, name: String) -> PKTNewPC {
    PKTNewPC { 
        pc_struct: PKTNewPCInner { 
            player_id,
            name,
            class_id,
            max_item_level: 1.0,
            character_id,
            stat_pairs: vec![],
            equip_item_datas: vec![],
            status_effect_datas: vec![]
        }
    }
}

