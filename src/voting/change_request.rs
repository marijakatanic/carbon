use crate::{discovery::Client, view::View, voting::{ResolutionClaim, Resignation}};

use doomstack::{ResultExt, here, Doom, Top};

pub(crate) enum ChangeRequest {
    ResolutionClaim(ResolutionClaim),
    Resignation(Resignation),
}

#[derive(Doom)]
pub(crate) enum ChangeRequestError {
    #[doom(description("`Resolution` invalid"))]
    ResolutionInvalid,
    #[doom(description("`Resignation` invalid"))]
    ResignationInvalid
}

impl ChangeRequest {
    pub fn validate(
        &self,
        client: &Client,
        current_view: &View,
    ) -> Result<(), Top<ChangeRequestError>> {
        match self {
            ChangeRequest::ResolutionClaim(resolution_claim) => {
                resolution_claim.validate(client, current_view).pot(ChangeRequestError::ResolutionInvalid, here!())
            }
            ChangeRequest::Resignation(resignation) => {
                resignation.validate(client, current_view).pot(ChangeRequestError::ResignationInvalid, here!())
            }
        }
    }
}