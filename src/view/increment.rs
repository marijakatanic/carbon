use crate::view::Change;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Increment {
    updates: Vec<Change>,
}

impl Increment {
    pub fn new<C>(&self, updates: C) -> Self
    where
        C: IntoIterator<Item = Change>,
    {
        let mut updates = updates.into_iter().collect::<Vec<_>>();
        updates.sort();

        Increment { updates }
    }

    pub(in crate::view) fn into_vec(self) -> Vec<Change> {
        self.updates
    }
}
