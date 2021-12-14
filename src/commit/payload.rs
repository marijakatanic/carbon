use crate::{
    account::{Entry, Id, Operation},
    crypto::Identify,
    prepare::Prepare,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Payload {
    entry: Entry,
    operation: Operation,
}

impl Payload {
    pub fn new(entry: Entry, operation: Operation) -> Self {
        Payload { entry, operation }
    }

    pub fn entry(&self) -> Entry {
        self.entry
    }

    pub fn id(&self) -> Id {
        self.entry.id
    }

    pub fn height(&self) -> u64 {
        self.entry.height
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    pub fn dependency(&self) -> Option<Entry> {
        self.operation.dependency()
    }

    pub fn prepare(&self) -> Prepare {
        Prepare::new(self.entry, self.operation.identifier())
    }
}
