use crate::account::Id;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CorruptedState {
    id: Id,
}

impl CorruptedState {
    pub fn new(id: Id) -> Self {
        CorruptedState { id }
    }
}
