use crate::view::Change;

use serde::{Deserialize, Serialize};

use std::cmp::Ordering;

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

        updates.sort_by(|lho, rho| match (lho, rho) {
            (Change::Join(_), Change::Leave(_)) => Ordering::Greater,
            (Change::Leave(_), Change::Join(_)) => Ordering::Less,
            (lho, rho) => lho.identity().cmp(&rho.identity()),
        });

        Increment { updates }
    }

    pub(in crate::view) fn into_vec(self) -> Vec<Change> {
        self.updates
    }
}
