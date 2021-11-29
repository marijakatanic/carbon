use crate::signup::IdClaim;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum BrokerFailure {
    Throttle,
    Error,
    Collision {
        brokered: IdClaim,
        collided: IdClaim,
    },
}
