use crate::view_generator::InstallPrecursor;

use talk::crypto::primitives::hash::Hash;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) enum SummarizeSend {
    Brief { precursor: Hash },
    Expanded { precursor: InstallPrecursor },
}
