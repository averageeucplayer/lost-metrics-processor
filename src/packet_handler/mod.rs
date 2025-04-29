mod on_counterattack;
mod on_death;
mod on_identity_change;
mod on_init_env;
mod on_init_pc;
mod on_new_pc;
mod on_new_npc;
mod on_new_npc_summon;
mod on_new_projectile;
mod on_new_trap;
mod on_raid_begin;
mod on_raid_boss_kill;
mod on_raid_result;
mod on_remove_object;
mod on_skill_cast;
mod on_skill_start;
mod on_skill_damage_abnormal;
mod on_skill_damage;
mod on_party_info;
mod on_party_leave;
mod on_party_status_effect_add;
mod on_party_status_effect_remove;
mod on_party_status_effect_result;
mod on_status_effect_add;
mod on_status_effect_remove;
mod on_trigger_boss_battle_status;
mod on_trigger_start;
mod on_zone_member_load;
mod on_zone_object_unpublish;
mod on_status_effect_sync;
mod on_troop_member_update;
mod on_new_transit;

use crate::encounter_state::EncounterState;
use super::{abstractions::*, StartOptions};
use anyhow::Ok;
use chrono::Utc;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
use std::sync::{Arc, RwLock};

use super::flags::Flags;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait PacketHandler {
    fn handle(&mut self, opcode: Pkt, data: &[u8], state: &mut EncounterState, options: &StartOptions) -> anyhow::Result<()>;
}

pub struct DefaultPacketHandler<FL, DH, SA, RS, LP, EE, PE>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    PE: Persister
{
    damage_encryption_handler: Arc<DH>,
    region_store: Arc<RS>,
    local_player_store: Arc<RwLock<LP>>,
    event_emitter: Arc<EE>,
    persister: Arc<PE>,
    stats_api: Arc<SA>,
    flags: Arc<FL>
}

impl<FL, DH, SA, RS, LP, EE, PE> PacketHandler for DefaultPacketHandler<FL, DH, SA, RS, LP, EE, PE>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    PE: Persister
{
    fn handle(
        &mut self,
        opcode: Pkt,
        data: &[u8],
        state: &mut EncounterState,
        options: &StartOptions) -> anyhow::Result<()> {
        let now = Utc::now();

        match opcode {
            Pkt::CounterAttackNotify => self.on_counterattack(data, state)?,
            Pkt::DeathNotify => self.on_death(now, data, state)?,
            Pkt::IdentityGaugeChangeNotify => self.on_identity_change(now ,data, state)?,
            Pkt::InitEnv => self.on_init_env(now, data, state, &options.version)?,
            Pkt::InitPC => self.on_init_pc(now, data, state)?,
            Pkt::NewPC => self.on_new_pc(now, data, state)?,
            Pkt::NewNpc => self.on_new_npc(now, data, state)?,
            Pkt::NewNpcSummon => self.on_new_npc_summon(now, data, state)?,
            Pkt::NewProjectile => self.on_new_projectile(data, state)?,
            Pkt::NewTrap => self.on_new_trap(data, state)?,
            Pkt::RaidBegin => self.on_raid_begin(data, state)?,
            Pkt::RaidBossKillNotify => self.on_raid_boss_kill(state, &options.version)?,
            Pkt::RaidResult => self.on_raid_result(now, state, &options.version)?,
            Pkt::RemoveObject => self.on_remove_object(data, state)?,
            Pkt::SkillCastNotify => self.on_skill_cast(now, data, state)?,
            Pkt::SkillStartNotify => self.on_skill_start(now, data, state)?,
            Pkt::SkillDamageAbnormalMoveNotify => self.on_skill_damage_abnormal(now, data, state, options)?,
            Pkt::SkillDamageNotify => self.on_skill_damage(now, data, state, options)?,
            Pkt::PartyInfo => self.on_party_info(data, state)?,
            Pkt::PartyLeaveResult => self.on_party_leave(data, state)?,
            Pkt::PartyStatusEffectAddNotify => self.on_party_status_effect_add(now, data, state)?,
            Pkt::PartyStatusEffectRemoveNotify => self.on_party_status_effect_remove(data, state)?,
            Pkt::PartyStatusEffectResultNotify => self.on_party_status_effect_result(data, state)?,
            Pkt::StatusEffectAddNotify => self.on_status_effect_add(now, data, state)?,
            Pkt::StatusEffectRemoveNotify => self.on_status_effect_remove(now, data, state)?,
            Pkt::TriggerBossBattleStatus => self.on_trigger_boss_battle_status(state, &options.version)?,
            Pkt::TriggerStartNotify => self.on_trigger_start(now, data, state, &options.version)?,
            Pkt::ZoneMemberLoadStatusNotify => self.on_zone_member_load(data, state)?,
            Pkt::ZoneObjectUnpublishNotify => self.on_zone_object_unpublish(data, state)?,
            Pkt::StatusEffectSyncDataNotify => self.on_status_effect_sync(data, state)?,
            Pkt::TroopMemberUpdateMinNotify => self.on_troop_member_update(data, state)?,
            Pkt::NewTransit => self.on_new_transit(data)?,
            _ => {}
        }

        Ok(())
    }
}

impl<FL, DH, SA, RS, LP, EE, PE> DefaultPacketHandler<FL, DH, SA, RS, LP, EE, PE>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    PE: Persister {
    pub fn new(
        flags: Arc<FL>,
        damage_encryption_handler: Arc<DH>,
        local_player_store: Arc<RwLock<LP>>,
        event_emitter: Arc<EE>,
        region_store: Arc<RS>,
        persister: Arc<PE>,
        stats_api: Arc<SA>) -> Self {
        

        Self {
            flags,
            damage_encryption_handler,
            local_player_store,
            event_emitter,
            region_store,
            persister,
            stats_api,
        }
    }
}