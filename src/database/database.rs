use crate::{
    account::Id,
    database::{Families, Signup},
    signup::IdAssignment,
};

use std::collections::{HashMap, HashSet};

use zebra::database::Family;

pub(crate) struct Database {
    pub assignments: HashMap<Id, IdAssignment>,
    pub signup: Signup,
    pub families: Families,
}

impl Database {
    pub fn new() -> Self {
        let families = Families { id: Family::new() };

        Database {
            assignments: HashMap::new(),
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
