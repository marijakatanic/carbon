use crate::{
    discovery::Client,
    view::View,
    voting::{Resignation, ResolutionClaim},
};

use doomstack::{here, Doom, ResultExt, Top};

pub(crate) enum Claim {
    ResolutionClaim(ResolutionClaim),
    Resignation(Resignation),
}

#[derive(Doom)]
pub(crate) enum ChangeRequestError {
    #[doom(description("`Resolution` invalid"))]
    ResolutionInvalid,
    #[doom(description("`Resignation` invalid"))]
    ResignationInvalid,
}

impl Claim {
    pub fn validate(
        &self,
        client: &Client,
        current_view: &View,
    ) -> Result<(), Top<ChangeRequestError>> {
        match self {
            Claim::ResolutionClaim(resolution_claim) => resolution_claim
                .validate(client, current_view)
                .pot(ChangeRequestError::ResolutionInvalid, here!()),
            Claim::Resignation(resignation) => resignation
                .validate(client, current_view)
                .pot(ChangeRequestError::ResignationInvalid, here!()),
        }
    }
}
