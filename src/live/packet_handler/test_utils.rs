use lost_metrics_sniffer_stub::packets::definitions::{PKTNewPC, PKTNewPCInner};

pub fn create_random_pc(player_id: u64, name: String) -> PKTNewPC {
    PKTNewPC { 
        pc_struct: PKTNewPCInner { 
            player_id,
            name,
            class_id: 101,
            max_item_level: 1.0,
            character_id: 1,
            stat_pairs: vec![],
            equip_item_datas: vec![],
            status_effect_datas: vec![]
        }
    }
}
