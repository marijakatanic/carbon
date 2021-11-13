use crate::{
    discovery::Client,
    view::{Change, View},
};

use doomstack::{Doom, Top};

use talk::crypto::primitives::sign::Signature;

pub(crate) struct Resignation(ResignationClaim);

pub(crate) struct ResignationClaim {
    statement: Statement,
    signature: Signature,
}

struct Statement {
    change: Change,
}

#[derive(Doom)]
pub(crate) enum ResignationError {
    #[doom(description("Invalid signature"))]
    SignatureInvalid,
    #[doom(description("The `Resignation`'s `Change` cannot be applied to the current `View`"))]
    ViewError,
}

impl Resignation {
    pub fn validate(
        &self,
        client: &Client,
        current_view: &View,
    ) -> Result<(), Top<ResignationError>> {
        todo!()
    }

    pub fn to_resignation() -> Result<(), Top<ResignationError>> {
        todo!()
    }
}
