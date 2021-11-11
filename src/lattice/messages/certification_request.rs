use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct CertificationRequest {
    pub elements: BTreeSet<Hash>,
}
