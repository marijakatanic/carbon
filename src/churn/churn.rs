use crate::{
    churn::{ResignationClaim, ResolutionClaim},
    discovery::Client,
    view::{Change, View},
};

use doomstack::{here, Doom, ResultExt, Top};

pub(crate) enum Churn {
    Resolution(ResolutionClaim),
    Resignation(ResignationClaim),
}

#[derive(Doom)]
pub(crate) enum ChurnError {
    #[doom(description("`Resolution` invalid"))]
    ResolutionInvalid,
    #[doom(description("`Resignation` invalid"))]
    ResignationInvalid,
}

impl Churn {
    pub fn validate(&self, client: &Client, current_view: &View) -> Result<(), Top<ChurnError>> {
        match self {
            Churn::Resolution(resolution_claim) => resolution_claim
                .validate(client, current_view)
                .pot(ChurnError::ResolutionInvalid, here!()),

            Churn::Resignation(resignation) => resignation
                .validate(current_view)
                .pot(ChurnError::ResignationInvalid, here!()),
        }
    }

    pub fn to_change(
        self,
        client: &Client,
        current_view: &View,
    ) -> Result<Change, Top<ChurnError>> {
        match self {
            Churn::Resolution(resolution_claim) => resolution_claim
                .to_resolution(client, current_view)
                .map(|resolution| resolution.change())
                .pot(ChurnError::ResolutionInvalid, here!()),

            Churn::Resignation(resignation_claim) => resignation_claim
                .to_resignation(current_view)
                .map(|resignation| resignation.change())
                .pot(ChurnError::ResignationInvalid, here!()),
        }
    }
}
