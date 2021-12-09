use crate::crypto::Identify;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::{self, Hash};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Operation {}

impl Identify for Operation {
    fn identifier(&self) -> Hash {
        hash::hash(self).unwrap()
    }
}
