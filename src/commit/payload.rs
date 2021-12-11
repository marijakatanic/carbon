use crate::{
    account::{Entry, Id, Operation},
    crypto::Identify,
    prepare::Prepare,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Payload {
    id: Id,
    height: u64,
    operation: Operation,
}

impl Payload {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn operation(&self) -> &Operation {
        &self.operation
    }

    pub fn entry(&self) -> Entry {
        Entry {
            id: self.id,
            height: self.height,
        }
    }

    pub fn dependency(&self) -> Option<Entry> {
        self.operation.dependency()
    }

    pub fn prepare(&self) -> Prepare {
        Prepare::new(self.id, self.height, self.operation.identifier())
    }
}
