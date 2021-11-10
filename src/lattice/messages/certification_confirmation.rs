use serde::{Deserialize, Serialize};
use talk::crypto::primitives::{hash::Hash, multi::Signature};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct CertificationConfirmation {
    pub identifier: Hash,     // Identifier of the decision
    pub signature: Signature, // Signature of the decision
}
