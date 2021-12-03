use crate::{
    account::Id,
    database::{
        prepare::{Advertisement, BatchHolder, State},
        Zebras,
    },
};

use std::collections::{HashMap, HashSet};

use talk::crypto::primitives::hash::Hash;

use zebra::database::Table;

pub(crate) struct Prepare {
    advertisements: Table<Id, Advertisement>,
    states: HashMap<Id, State>,
    stale: HashSet<Id>,
    batches: HashMap<Hash, BatchHolder>,
}

impl Prepare {
    pub fn new(zebras: &Zebras) -> Self {
        Prepare {
            advertisements: zebras.ids_to_prepare_advertisements.empty_table(),
            states: HashMap::new(),
            stale: HashSet::new(),
            batches: HashMap::new(),
        }
    }
}
