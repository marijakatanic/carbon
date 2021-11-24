use crate::signup::IdAllocation;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum SignupResponse {
    IdAllocations(Vec<IdAllocation>),
    IdAssignments(Vec<Option<MultiSignature>>),
}
