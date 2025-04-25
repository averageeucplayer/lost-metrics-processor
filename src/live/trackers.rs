// use std::{cell::RefCell, rc::Rc};

// use hashbrown::HashMap;

// use super::{entity_tracker::EntityTracker, id_tracker::IdTracker, party_tracker::PartyTracker, status_tracker::StatusTracker};

// #[derive(Debug)]
// pub struct Trackers {
//     pub id_tracker: Rc<RefCell<IdTracker>>,
//     pub party_tracker: Rc<RefCell<PartyTracker>>,
//     pub status_tracker: Rc<RefCell<StatusTracker>>,
//     pub entity_tracker: EntityTracker
// }

// impl Trackers {
//     pub fn new() -> Self {
//         let id_tracker: Rc<RefCell<IdTracker>> = Rc::new(RefCell::new(IdTracker::new()));
//         let party_tracker: Rc<RefCell<PartyTracker>> = Rc::new(RefCell::new(PartyTracker::new(id_tracker.clone())));
//         let status_tracker: Rc<RefCell<StatusTracker>> = Rc::new(RefCell::new(StatusTracker::new(party_tracker.clone())));
//         let entity_tracker = EntityTracker::new(
//             status_tracker.clone(),
//             id_tracker.clone(),
//             party_tracker.clone(),
//         );
    
//         Self {
//             id_tracker,
//             party_tracker,
//             status_tracker,
//             entity_tracker   
//         }
//     }

//     pub fn get_party_from_tracker(&self) -> Vec<Vec<String>> {
//         let entity_id_to_party_id = &self.party_tracker.borrow().entity_id_to_party_id;
//         let entities = &self.entity_tracker.entities;
//         let mut party_info: HashMap<u32, Vec<String>> = HashMap::new();

//         for (entity_id, party_id) in entity_id_to_party_id.iter() {
//             let entity_name = entities.get(entity_id).map(|entity| entity.name.clone());
//             party_info.entry(*party_id)
//                 .or_insert_with(Vec::new)
//                 .extend(entity_name);
//         }
        
//         let mut sorted_parties = party_info.into_iter().collect::<Vec<(u32, Vec<String>)>>();
//         sorted_parties.sort_by_key(|&(party_id, _)| party_id);

//         sorted_parties
//             .into_iter()
//             .map(|(_, members)| members)
//             .collect()
//     }
// }