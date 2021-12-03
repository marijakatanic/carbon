use crate::{
    account::{Id, State},
    database::Signup,
    signup::IdAssignment,
};

use std::collections::{HashMap, HashSet};

use zebra::database::Family;

pub(crate) struct Database {
    pub assignments: HashMap<Id, IdAssignment>,
    pub states: HashMap<Id, State>,

    pub signup: Signup,

    pub families: Families,
}

pub(crate) struct Families {
    pub id: Family<Id>,
}

impl Database {
    pub fn new() -> Self {
        let families = Families { id: Family::new() };

        Database {
            assignments: HashMap::new(),
            states: HashMap::new(),
            signup: Signup {
                allocations: HashMap::new(),
                allocated: HashSet::new(),

                claimed: families.id.empty_collection(),
                claims: HashMap::new(),
            },
            families,
        }
    }
}
