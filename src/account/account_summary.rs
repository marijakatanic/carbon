use crate::account::StateSummary;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AccountSummary {
    pub height: u64,
    pub state: StateSummary,
}
