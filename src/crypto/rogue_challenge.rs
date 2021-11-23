use crate::crypto::Header;

use serde::Serialize;

use talk::crypto::Statement;

#[derive(Serialize)]
pub(crate) struct RogueChallenge;

impl Statement for RogueChallenge {
    type Header = Header;
    const HEADER: Header = Header::RogueChallenge;
}
