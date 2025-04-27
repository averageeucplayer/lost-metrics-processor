mod templates;
mod packet_builder;
mod packet_handler_builder;
mod player_templates;
pub mod state_builder;

pub use player_templates::*;
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

pub fn to_status_effect_value(value: u64) -> Option<Vec<u8>> {
    if value == 0 {
        return None;
    }

    let mut buffer = Vec::with_capacity(16);
    buffer.extend_from_slice(&value.to_le_bytes()); // first 8 bytes
    buffer.extend_from_slice(&value.to_le_bytes()); // second 8 bytes
    Some(buffer)
}