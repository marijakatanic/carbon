use crate::account::Entry;

use serde::{Deserialize, Serialize};

use zebra::map::Set;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Deposit {
    withdraw: Entry,
    exclusion: Option<Set<Entry>>,
    collect: bool,
}

impl Deposit {
    pub fn new(withdraw: Entry, deposits: Option<&Set<Entry>>, collect: bool) -> Self {
        let exclusion = deposits.map(|deposits| deposits.export([&withdraw]).unwrap());

        Deposit {
            withdraw,
            exclusion,
            collect,
        }
    }

    pub fn withdraw(&self) -> Entry {
        self.withdraw
    }

    pub fn exclusion(&self) -> &Option<Set<Entry>> {
        &self.exclusion
    }

    pub fn collect(&self) -> bool {
        self.collect
    }

    pub fn dependency(&self) -> Option<Entry> {
        Some(self.withdraw)
    }
}
