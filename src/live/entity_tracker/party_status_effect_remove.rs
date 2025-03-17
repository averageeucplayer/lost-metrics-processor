use lost_metrics_core::models::{StatusEffectDetails, StatusEffectTargetType};
use lost_metrics_sniffer_stub::packets::definitions::PKTPartyStatusEffectRemoveNotify;

use super::EntityTracker;

impl EntityTracker {

    pub fn party_status_effect_remove(
        &mut self,
        pkt: PKTPartyStatusEffectRemoveNotify,
    ) -> (
        bool,
        Vec<StatusEffectDetails>,
        Vec<StatusEffectDetails>,
        bool,
    ) {
        self.status_tracker.borrow_mut().remove_status_effects(
            pkt.character_id,
            pkt.status_effect_instance_ids,
            pkt.reason,
            StatusEffectTargetType::Party,
        )
    }
}