use crate::lattice::lattice_runner::Decision;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::lattice) struct CertificationRequest<Instance> {
    pub decision: Decision<Instance>,
}
