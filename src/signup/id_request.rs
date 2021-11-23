use crate::{
    crypto::{Header, Identify, RogueChallenge},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash::Hash;
use talk::crypto::primitives::multi::Signature as MultiSignature;
use talk::crypto::primitives::sign::Signature;
use talk::crypto::primitives::work::Work;
use talk::crypto::{Identity, KeyCard, KeyChain, Statement};

pub(crate) struct IdRequest {
    request: Request,
    work: Work,
    rogue: Rogue,
}

#[derive(Serialize, Deserialize)]
struct Request {
    keycard: KeyCard,
    view: Hash,
    assigner: Identity,
}

#[derive(Serialize, Deserialize)]
struct Rogue {
    sign: Signature,
    multi: MultiSignature,
}

#[derive(Doom)]
pub(crate) enum RequestIdError {
    #[doom(description("Foreign view"))]
    ForeignView,
    #[doom(description("Foreign verifier"))]
    ForeignVerifier,
    #[doom(description("Work invalid"))]
    WorkInvalid,
    #[doom(description("Rogue-safety proof invalid"))]
    RogueInvalid,
}

impl IdRequest {
    pub fn new(keychain: &KeyChain, view: &View, assigner: Identity) -> Self {
        let keycard = keychain.keycard();
        let view = view.identifier();

        let request = Request {
            keycard,
            view,
            assigner,
        };

        let work = Work::new(10, &request).unwrap(); // TODO: Add settings

        let rogue = Rogue {
            sign: keychain.sign(&RogueChallenge).unwrap(),
            multi: keychain.multisign(&RogueChallenge).unwrap(),
        };

        IdRequest {
            request,
            work,
            rogue,
        }
    }

    pub fn validate(&self, view: &View, verifier: Identity) -> Result<(), Top<RequestIdError>> {
        if self.request.view != view.identifier() {
            return RequestIdError::ForeignView.fail().spot(here!());
        }

        if self.request.assigner != verifier {
            return RequestIdError::ForeignVerifier.fail().spot(here!());
        }

        self.work
            .verify(10, &self.request)
            .pot(RequestIdError::WorkInvalid, here!())?;

        self.rogue
            .sign
            .verify(&self.request.keycard, &RogueChallenge)
            .pot(RequestIdError::RogueInvalid, here!())?;

        self.rogue
            .multi
            .verify([&self.request.keycard], &RogueChallenge)
            .pot(RequestIdError::RogueInvalid, here!())?;

        Ok(())
    }
}

impl Statement for Request {
    type Header = Header;
    const HEADER: Header = Header::IdRequest;
}
