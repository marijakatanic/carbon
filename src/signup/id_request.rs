use crate::{
    crypto::{Header, Identify, Rogue},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::{hash::Hash, work::Work},
    Identity, KeyCard, KeyChain, Statement,
};

#[derive(Serialize, Deserialize)]
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
        let rogue = Rogue::new(keychain);

        IdRequest {
            request,
            work,
            rogue,
        }
    }

    pub fn identity(&self) -> Identity {
        self.request.keycard.identity()
    }

    pub fn view(&self) -> Hash {
        self.request.view
    }

    pub fn assigner(&self) -> Identity {
        self.request.assigner
    }

    pub fn validate(&self, view: &View, assigner: Identity) -> Result<(), Top<RequestIdError>> {
        if self.request.view != view.identifier() {
            return RequestIdError::ForeignView.fail().spot(here!());
        }

        if self.request.assigner != assigner {
            return RequestIdError::ForeignVerifier.fail().spot(here!());
        }

        self.work
            .verify(10, &self.request)
            .pot(RequestIdError::WorkInvalid, here!())?;

        self.rogue
            .validate(&self.request.keycard)
            .pot(RequestIdError::RogueInvalid, here!())?;

        Ok(())
    }
}

impl Statement for Request {
    type Header = Header;
    const HEADER: Header = Header::IdRequest;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::view::test::InstallGenerator;

    #[test]
    fn correct() {
        let install_generator = InstallGenerator::new(4);

        let view = install_generator.view(4);
        let assigner = install_generator.keycards[0].identity();

        let keychain = KeyChain::random();

        let id_request = IdRequest::new(&keychain, &view, assigner);
        id_request.validate(&view, assigner).unwrap();
    }
}
