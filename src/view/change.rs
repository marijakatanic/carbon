use serde::{Deserialize, Serialize};

use std::cmp::{Ord, Ordering, PartialOrd};
use std::hash::Hash;

use talk::crypto::{Identity, KeyCard};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum Change {
    Join(KeyCard),
    Leave(KeyCard),
}

impl Change {
    pub fn mirror(self) -> Self {
        match self {
            Change::Join(replica) => Change::Leave(replica),
            Change::Leave(replica) => Change::Join(replica),
        }
    }

    pub fn identity(&self) -> Identity {
        match self {
            Change::Join(card) => card.identity(),
            Change::Leave(card) => card.identity(),
        }
    }

    pub fn is_join(&self) -> bool {
        match self {
            Change::Join(_) => true,
            Change::Leave(_) => false,
        }
    }

    pub fn is_leave(&self) -> bool {
        match self {
            Change::Join(_) => false,
            Change::Leave(_) => true,
        }
    }
}

impl PartialOrd for Change {
    fn partial_cmp(&self, rho: &Change) -> Option<Ordering> {
        Some(self.cmp(rho))
    }
}

impl Ord for Change {
    fn cmp(&self, rho: &Change) -> Ordering {
        match (self, rho) {
            (Change::Join(_), Change::Leave(_)) => Ordering::Greater,
            (Change::Leave(_), Change::Join(_)) => Ordering::Less,
            (lho, rho) => lho.identity().cmp(&rho.identity()),
        }
    }
}
