use crate::signup::IdClaim;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Failure {
    Network,
    Collision {
        brokered: IdClaim,
        collided: IdClaim,
    },
}
