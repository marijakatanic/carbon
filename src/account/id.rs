use crate::crypto::Identify;

use talk::crypto::primitives::hash::{self, Hash};

pub(crate) type Id = u64;

impl Identify for Id {
    fn identifier(&self) -> Hash {
        hash::hash(&self).unwrap()
    }
}
