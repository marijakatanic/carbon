use crate::signup::IdAllocation;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum SignupResponse {
    IdAllocations(Vec<IdAllocation>),
}
