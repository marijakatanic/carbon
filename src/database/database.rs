use buckets::Buckets;

use crate::{
    account::{Account, AccountSummary, Id},
    database::{Commit, Prepare, Signup, Zebras},
    signup::IdAssignment,
};

use std::collections::HashMap;

use zebra::database::Table;

pub(crate) struct Database {
    pub assignments: Buckets<HashMap<Id, IdAssignment>>,
    pub accounts: Buckets<HashMap<Id, Account>>,
    pub imminent: Table<Id, AccountSummary>,

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
            accounts: Buckets::new(),
            imminent: zebras.ids_to_account_summaries.empty_table(),

            signup: Signup::new(&zebras),
            prepare: Prepare::new(&zebras),
            commit: Commit::new(),

            families: zebras,
        }
    }
}
