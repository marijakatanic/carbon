use crate::signup::IdRequest;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum SignupRequest {
    IdRequests(Vec<IdRequest>)
}