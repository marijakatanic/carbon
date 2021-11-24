use crate::crypto::Header;

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::multi::Signature as MultiSignature;
use talk::crypto::primitives::sign::Signature;
use talk::crypto::{KeyCard, KeyChain, Statement};

#[derive(Serialize, Deserialize)]
pub(crate) struct Rogue {
    sign: Signature,
    multi: MultiSignature,
}

#[derive(Serialize)]
struct RogueChallenge;

#[derive(Doom)]
pub(crate) enum RogueError {
    #[doom(description("Invalid `Signature`"))]
    InvalidSignature,
    #[doom(description("Invalid `MultiSignature`"))]
    InvalidMultiSignature,
}

impl Rogue {
    pub fn new(keychain: &KeyChain) -> Self {
        Rogue {
            sign: keychain.sign(&RogueChallenge).unwrap(),
            multi: keychain.multisign(&RogueChallenge).unwrap(),
        }
    }

    pub fn validate(&self, keycard: &KeyCard) -> Result<(), Top<RogueError>> {
        self.sign
            .verify(&keycard, &RogueChallenge)
            .pot(RogueError::InvalidSignature, here!())?;

        self.multi
            .verify([keycard], &RogueChallenge)
            .pot(RogueError::InvalidMultiSignature, here!())
    }
}

impl Statement for RogueChallenge {
    type Header = Header;
    const HEADER: Header = Header::RogueChallenge;
}
