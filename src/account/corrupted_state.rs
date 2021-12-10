use crate::account::Id;

pub(crate) struct CorruptedState {
    id: Id,
}

impl CorruptedState {
    pub fn new(id: Id) -> Self {
        CorruptedState { id }
    }
}
