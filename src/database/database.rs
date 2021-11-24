use crate::{
    account::{Id, State},
    signup::{IdAssignment, IdClaim},
};

use std::collections::{HashMap, HashSet};

use talk::crypto::Identity;

use zebra::database::{Collection, Family};

pub(crate) struct Database {
    pub assignments: HashMap<Id, IdAssignment>,
    pub states: HashMap<Id, State>,

    pub signup: Signup,

    pub families: Families,
}

pub(crate) struct Signup {
    pub allocated: HashSet<Id>,
    pub allocations: HashMap<Identity, Id>,

    // TODO: Include in state-transfer {
    pub claimed: Collection<Id>,
    pub claims: HashMap<Id, IdClaim>,
    // }
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
