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
    pub advertisements: Table<Id, Advertisement>,
    pub advertised: HashSet<Id>,
    pub states: HashMap<Id, State>,
    pub stale: HashSet<Id>,
    pub batches: HashMap<Hash, BatchHolder>,
}

impl Prepare {
    pub fn new(zebras: &Zebras) -> Self {
        Prepare {
            advertisements: zebras.ids_to_prepare_advertisements.empty_table(),
            advertised: HashSet::new(),
            states: HashMap::new(),
            stale: HashSet::new(),
            batches: HashMap::new(),
        }
    }
}
