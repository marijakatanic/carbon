use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::Identity;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct DisclosureReady {
    pub origin: Identity,
    pub disclosure: Hash,
}
