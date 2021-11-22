use crate::view_generator::messages::{SummarizationRequest, SummarizationResponse};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) enum Message {
    SummarizationRequest(SummarizationRequest),
    SummarizationResponse(SummarizationResponse),
}
