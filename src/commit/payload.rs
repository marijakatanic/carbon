use crate::{
    account::{Id, Operation},
    crypto::Identify,
    prepare::Prepare,
};

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

    pub fn prepare(&self) -> Prepare {
        Prepare::new(self.id, self.height, self.operation.identifier())
    }
}
