use buckets::Buckets;

use crate::{
    account::Id,
    database::{Commit, Prepare, Signup, Zebras},
    signup::IdAssignment,
};

use std::collections::HashMap;

pub(crate) struct Database {
    pub assignments: Buckets<HashMap<Id, IdAssignment>>,
    pub signup: Signup,
    pub prepare: Prepare,
    pub commit: Commit,
    pub families: Zebras,
}

impl Database {
    pub fn new() -> Self {
        let zebras = Zebras::new();

        Database {
            assignments: Buckets::new(),
            signup: Signup::new(&zebras),
            prepare: Prepare::new(&zebras),
            commit: Commit::new(),
            families: zebras,
        }
    }
}
