use talk::crypto::primitives::hash::Hash;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Advertisement {
    Consistent { height: u64, commitment: Hash },
    Equivocated,
}
