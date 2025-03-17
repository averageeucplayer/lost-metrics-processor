use log::warn;
use lost_metrics_core::models::{EntityType, LocalInfo, LocalPlayer};
use lost_metrics_sniffer_stub::packets::definitions::PKTPartyInfo;

use crate::live::entity_tracker::truncate_gear_level;

use super::EntityTracker;

impl EntityTracker {
        
    pub fn party_info(&mut self, pkt: PKTPartyInfo, local_info: &LocalInfo) {
        let mut unknown_local = if let Some(local_player) = self.entities.get(&self.local_entity_id)
        {
            local_player.name.is_empty()
                || local_player.name == "You"
                || local_player.name.starts_with('0')
        } else {
            true
        };

        self.party_tracker
            .borrow_mut()
            .remove_party_mappings(pkt.party_instance_id);

        let most_likely_local_name = if unknown_local {
            let party_members = pkt
                .party_member_datas
                .iter()
                .map(|m| m.character_id)
                .collect::<Vec<u64>>();
            let mut party_locals = local_info
                .local_players
                .iter()
                .filter_map(|(k, v)| {
                    if party_members.contains(k) {
                        Some(v)
                    } else {
                        None
                    }
                })
                .collect::<Vec<&LocalPlayer>>();
            party_locals.sort_by(|a, b| b.count.cmp(&a.count));
            party_locals
                .first()
                .map_or_else(String::new, |p| p.name.clone())
        } else {
            "".to_string()
        };

        for member in pkt.party_member_datas {
            if unknown_local && member.name == most_likely_local_name {
                if let Some(local_player) = self.entities.get_mut(&self.local_entity_id) {
                    unknown_local = false;
                    warn!(
                        "unknown local player, inferring from cache: {}",
                        member.name
                    );
                    local_player.entity_type = EntityType::Player;
                    local_player.class_id = member.class_id as u32;
                    local_player.gear_level = truncate_gear_level(member.gear_level);
                    local_player.name.clone_from(&member.name);
                    local_player.character_id = member.character_id;
                    self.id_tracker
                        .borrow_mut()
                        .add_mapping(member.character_id, self.local_entity_id);
                    self.party_tracker
                        .borrow_mut()
                        .set_name(member.name.clone());
                }
            }

            let entity_id = self.id_tracker.borrow().get_entity_id(member.character_id);

            if let Some(entity_id) = entity_id {
                if let Some(entity) = self.entities.get_mut(&entity_id) {
                    if entity.entity_type == EntityType::Player && entity.name == member.name {
                        entity.gear_level = truncate_gear_level(member.gear_level);
                        entity.class_id = member.class_id as u32;
                    }
                }

                self.party_tracker.borrow_mut().add(
                    pkt.raid_instance_id,
                    pkt.party_instance_id,
                    member.character_id,
                    entity_id,
                    Some(member.name.clone()),
                );
            } else {
                self.party_tracker.borrow_mut().add(
                    pkt.raid_instance_id,
                    pkt.party_instance_id,
                    member.character_id,
                    0,
                    Some(member.name.clone()),
                );
            }
        }
    }
}