use crate::{
    crypto::{Header, Identify},
    view::{Change, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;
use talk::crypto::primitives::sign::Signature;
use talk::crypto::{KeyCard, Statement as CryptoStatement};

#[derive(Clone, Serialize)]
#[serde(into = "ResignationClaim")]
pub(crate) struct Resignation(ResignationClaim);

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ResignationClaim {
    member: KeyCard,
    statement: Statement,
    signature: Signature,
}

#[derive(Clone, Serialize, Deserialize)]
struct Statement {}

#[derive(Doom)]
pub(crate) enum ResignationError {
    #[doom(description("The `Resignation` is incorrectly signed"))]
    SignatureInvalid,
    #[doom(description("The `Resignation`'s `Change` cannot be applied to the current `View`"))]
    ViewError,
}

impl Resignation {
    pub fn change(&self) -> Change {
        Change::Leave(self.0.member.clone())
    }
}

impl ResignationClaim {
    pub fn validate(&self, current_view: &View) -> Result<(), Top<ResignationError>> {
        // Verify that `Change::Leave(self.member)` can be used to extend `current_view`
        let change = Change::Leave(self.member.clone());

        current_view
            .validate_extension(&change)
            .pot(ResignationError::ViewError, here!())?;

        // Verify `self.signature`
        self.signature
            .verify(&self.member, &self.statement)
            .pot(ResignationError::SignatureInvalid, here!())?;

        Ok(())
    }

    pub fn to_resignation(self, current_view: &View) -> Result<Resignation, Top<ResignationError>> {
        self.validate(current_view)?;
        Ok(Resignation(self))
    }
}

impl Identify for Resignation {
    fn identifier(&self) -> Hash {
        self.0.identifier()
    }
}

impl From<Resignation> for ResignationClaim {
    fn from(resignation: Resignation) -> Self {
        resignation.0
    }
}

impl Identify for ResignationClaim {
    fn identifier(&self) -> Hash {
        let change = Change::Leave(self.member.clone());
        hash::hash(&change).unwrap()
    }
}

impl CryptoStatement for Statement {
    type Header = Header;
    const HEADER: Header = Header::Resignation;
}
