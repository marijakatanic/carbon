use crate::signup::{IdAssignment, IdClaim, IdRequest};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum SignupRequest {
    IdRequests(Vec<IdRequest>),
    IdClaims(Vec<IdClaim>),
    IdAssignments(Vec<IdAssignment>),
}

impl SignupRequest {
    pub fn unwrap_id_requests(self) -> Vec<IdRequest> {
        match self {
            SignupRequest::IdRequests(id_requests) => id_requests,
            _ => panic!(
                "called `unwrap_id_requests` on a variant other than `SignupRequest::IdRequests`"
            ),
        }
    }

    pub fn unwrap_id_claims(self) -> Vec<IdClaim> {
        match self {
            SignupRequest::IdClaims(id_claims) => id_claims,
            _ => panic!(
                "called `unwrap_id_claims` on a variant other than `SignupRequest::IdClaims`"
            ),
        }
    }

    pub fn unwrap_id_assignments(self) -> Vec<IdAssignment> {
        match self {
            SignupRequest::IdAssignments(id_assignments) => id_assignments,
            _ => panic!(
                "called `unwrap_id_assignments` on a variant other than `SignupRequest::IdAssignments`"
            ),
        }
    }
}
