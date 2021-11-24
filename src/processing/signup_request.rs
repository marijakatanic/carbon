use crate::signup::{IdClaim, IdRequest};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum SignupRequest {
    IdRequests(Vec<IdRequest>),
    IdClaims(Vec<IdClaim>),
}
