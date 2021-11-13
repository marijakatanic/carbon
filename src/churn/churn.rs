use crate::{
    churn::{Resignation, ResolutionClaim},
    discovery::Client,
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

pub(crate) enum Churn {
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

impl Churn {
    pub fn validate(
        &self,
        client: &Client,
        current_view: &View,
    ) -> Result<(), Top<ChangeRequestError>> {
        match self {
            Churn::ResolutionClaim(resolution_claim) => resolution_claim
                .validate(client, current_view)
                .pot(ChangeRequestError::ResolutionInvalid, here!()),
            Churn::Resignation(resignation) => resignation
                .validate(client, current_view)
                .pot(ChangeRequestError::ResignationInvalid, here!()),
        }
    }
}
