use crate::signup::{IdAllocation, IdClaim};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;

#[derive(Serialize, Deserialize)]
pub(crate) enum SignupResponse {
    IdAllocations(Vec<IdAllocation>),
    IdAssignments(Vec<Result<MultiSignature, IdClaim>>),
}
