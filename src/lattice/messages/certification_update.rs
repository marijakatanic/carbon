use serde::{Deserialize, Serialize};
use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct CertificationUpdate {
    // With respect to a particular CertificationRequest
    pub identifier: Hash,       // Identifier of the decision
    pub differences: Vec<Hash>, // Missing elements
}
