use crate::view::Change;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Increment {
    updates: Vec<Change>,
}

impl Increment {
    pub fn new<C>(updates: C) -> Self
    where
        C: IntoIterator<Item = Change>,
    {
        let mut updates = updates.into_iter().collect::<Vec<_>>();

        #[cfg(debug_assertions)]
        {
            use std::collections::HashSet;

            let identities = updates.iter().map(Change::identity).collect::<HashSet<_>>();

            if identities.len() < updates.len() {
                panic!("Called `Increment::new` with non-distinct identities");
            }
        }

        updates.sort_by_key(Change::identity);

        Increment { updates }
    }

    pub(in crate::view) fn into_vec(self) -> Vec<Change> {
        self.updates
    }
}
