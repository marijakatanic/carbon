use crate::{
    account::{Entry, Id},
    discovery::Client,
    prepare::Prepare,
    signup::IdAssignment,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{
    primitives::{hash::Hash, sign::Signature},
    KeyCard, KeyChain,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Request {
    pub assignment: IdAssignment,
    pub prepare: Prepare,
    pub signature: Signature,
}

#[derive(Doom)]
pub(crate) enum RequestError {
    #[doom(description("`IdAssignment`'s `Id` does not match `Prepare`'s `Id`"))]
    IdsMismatched,
    #[doom(description("`IdAssignment` invalid"))]
    AssignmentInvalid,
    #[doom(description("`Signature` invalid"))]
    SignatureInvalid,
}

impl Request {
    pub fn new(
        keychain: &KeyChain,
        assignment: IdAssignment,
        height: u64,
        commitment: Hash,
    ) -> Self {
        let prepare = Prepare::new(
            Entry {
                id: assignment.id(),
                height,
            },
            commitment,
        );
        let signature = keychain.sign(&prepare).unwrap();

        Request {
            assignment,
            prepare,
            signature,
        }
    }

    pub fn id(&self) -> Id {
        self.assignment.id()
    }

    pub fn keycard(&self) -> &KeyCard {
        self.assignment.keycard()
    }

    pub fn prepare(&self) -> &Prepare {
        &self.prepare
    }

    pub fn validate(&self, discovery: &Client) -> Result<(), Top<RequestError>> {
        if self.assignment.id() != self.prepare.id() {
            return RequestError::IdsMismatched.fail().spot(here!());
        }

        self.assignment
            .validate(&discovery)
            .pot(RequestError::AssignmentInvalid, here!())?;

        self.signature
            .verify(&self.assignment.keycard(), &self.prepare)
            .pot(RequestError::SignatureInvalid, here!())?;

        Ok(())
    }
}
