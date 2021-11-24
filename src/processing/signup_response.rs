use crate::signup::IdAllocation;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum SignupResponse {
    IdAllocations(Vec<IdAllocation>),
}
