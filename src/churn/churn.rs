use crate::{
    churn::{ResignationClaim, ResolutionClaim},
    crypto::Identify,
    discovery::Client,
    view::{Change, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::cmp::{Ord, Ordering, PartialOrd};

use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
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

    fn change(&self) -> Change {
        match self {
            Churn::Resolution(resolution_claim) => resolution_claim.change(),
            Churn::Resignation(resignation_claim) => resignation_claim.change(),
        }
    }
}

impl PartialEq for Churn {
    fn eq(&self, rho: &Self) -> bool {
        self.change() == rho.change()
    }
}

impl Eq for Churn {}

impl PartialOrd for Churn {
    fn partial_cmp(&self, rho: &Self) -> Option<Ordering> {
        Some(self.cmp(rho))
    }
}

impl Ord for Churn {
    fn cmp(&self, rho: &Self) -> Ordering {
        self.change().cmp(&rho.change())
    }
}

impl Identify for Churn {
    fn identifier(&self) -> Hash {
        match self {
            Churn::Resolution(resolution_claim) => resolution_claim.identifier(),
            Churn::Resignation(resignation_claim) => resignation_claim.identifier(),
        }
    }
}
