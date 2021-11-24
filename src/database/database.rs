use crate::{
    account::{Id, State},
    signup::IdAllocation,
};

use std::collections::{HashMap, HashSet};

use talk::crypto::{Identity, KeyCard};

pub(crate) struct Database {
    pub keycards: HashMap<Id, KeyCard>,
    pub states: HashMap<Id, State>,

    pub signup: Signup,
}

pub(crate) struct Signup {
    pub assignments: HashMap<Identity, IdAllocation>,
    pub assigned: HashSet<Id>,
}

impl Database {
    pub fn new() -> Self {
        Database {
            keycards: HashMap::new(),
            states: HashMap::new(),
            signup: Signup {
                assignments: HashMap::new(),
                assigned: HashSet::new(),
            },
        }
    }
}
