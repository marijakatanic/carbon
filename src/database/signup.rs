use crate::{account::Id, database::Families, signup::IdClaim};

use std::collections::{HashMap, HashSet};

use talk::crypto::Identity;

use zebra::database::Collection;

pub(crate) struct Signup {
    pub allocated: HashSet<Id>,
    pub allocations: HashMap<Identity, Id>,

    // TODO: Include in state-transfer {
    pub claimed: Collection<Id>,
    pub claims: HashMap<Id, IdClaim>,
    // }
}

impl Signup {
    pub fn new(families: &Families) -> Self {
        Signup {
            allocated: HashSet::new(),
            allocations: HashMap::new(),
            claimed: families.id.empty_collection(),
            claims: HashMap::new(),
        }
    }
}
