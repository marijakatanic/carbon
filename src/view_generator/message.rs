use crate::view_generator::messages::{SummarizeConfirm, SummarizeSend};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) enum Message {
    SummarizeSend(SummarizeSend),
    SummarizeConfirm(SummarizeConfirm),
}
