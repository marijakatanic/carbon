use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct CertificationUpdate {
    pub identifier: Hash,            // Identifier of the decision
    pub differences: BTreeSet<Hash>, // Missing elements
}
