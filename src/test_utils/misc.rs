use std::{cell::RefCell, rc::Rc};

use chrono::{Duration, Utc};
use lost_metrics_core::models::*;
use lost_metrics_sniffer_stub::packets::structures::SkillDamageEvent;
use lost_metrics_store::encounter_service::EncounterService;

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

use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
#[cfg(test)]
use mockall::mock;

#[cfg(test)]
use lost_metrics_store::models::CreateEncounter;

use crate::{encounter_state::EncounterState, StartOptions};

#[cfg(test)]
mock! {
    pub DamageEncryptionHandlerTrait {}
    impl DamageEncryptionHandlerTrait for DamageEncryptionHandlerTrait {
        fn start(&self) -> anyhow::Result<()>;
        fn decrypt_damage_event(&self, event: &mut SkillDamageEvent) -> bool;
        fn update_zone_instance_id(&self, channel_id: u32);
    }
}


#[cfg(test)]
mock! {
    pub EncounterService {}
    impl EncounterService for EncounterService {
        fn create(&self, payload: CreateEncounter) -> anyhow::Result<i64>;
    }
}

pub fn create_start_options() -> StartOptions {
    StartOptions {
        version: "0.0.1".into(),
        port: 420,
        database_path: "encounter.db".into(),
        local_player_path: "local_players.json".into(),
        raid_end_capture_timeout: Duration::seconds(10),
        region_path: "current_region".into(),
        duration: Duration::milliseconds(500),
        party_duration: Duration::milliseconds(200),
    }
}

pub fn create_player_stats() -> EncounterEntity {
    use lost_metrics_core::models::{DamageStats, EntityType};

    let entity = EncounterEntity {
        id: 1,
        character_id: 1,
        name: "test".into(),
        entity_type: EntityType::Player,
        class_id: 101,
        damage_stats: DamageStats {
            damage_dealt: 1,
            ..Default::default()    
        },
        ..Default::default()
    };

    entity
}

pub fn update_state_with_player_and_boss(state: &mut EncounterState) {
    state.started_on = Utc::now();
    
    let player = EncounterEntity {
        id: 1,
        entity_type: EntityType::Player,
        name: "test_player".into(),
        damage_stats: DamageStats {
            damage_dealt: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    
    state.entity_stats.insert(player.id, player);

    let boss_stats = EncounterEntity {
        id: 2,
        entity_type: EntityType::Boss,
        name: "test_boss".into(),
        current_hp: 0,
        max_hp: 1e9 as i64,
        ..Default::default()
    };
    let boss = Entity {
        id: 2,
        entity_type: EntityType::Boss,
        ..Default::default()
    };

    state.entity_stats.insert(boss.id, boss_stats);
    let boss = Rc::new(RefCell::new(boss));
    state.current_boss = Some(boss);
   
}
