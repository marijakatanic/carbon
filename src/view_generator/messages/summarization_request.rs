use crate::view_generator::InstallPrecursor;

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) enum SummarizationRequest {
    Brief { precursor: Hash },
    Expanded { precursor: InstallPrecursor },
}
