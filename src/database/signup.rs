use crate::{account::Id, signup::IdClaim};

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
