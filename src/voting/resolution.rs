use crate::{
    crypto::{Certificate, Header},
    discovery::Client,
    view::{Change, View},
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::hash::Hash, Statement as CryptoStatement};

#[derive(Clone, Serialize)]
#[serde(into = "ResolutionClaim")]
pub(crate) struct Resolution(ResolutionClaim);

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ResolutionClaim {
    view: Hash,
    statement: Statement,
    certificate: Certificate,
}

#[derive(Clone, Serialize, Deserialize)]
struct Statement {
    change: Change,
}

#[derive(Doom)]
pub(crate) enum ResolutionError {
    #[doom(description("The `Resolution` pertains to an unknown `View`"))]
    UnknownView,
    #[doom(description("The `Resolution` did not pass in a past or current `View`"))]
    FutureVote,
    #[doom(description("Certificate invalid"))]
    CertificateInvalid,
    #[doom(description("The `Resolution`'s `Change` cannot be applied to the current `View`"))]
    ViewError,
}

impl Resolution {
    pub fn view(&self) -> Hash {
        self.0.view
    }

    pub fn change(&self) -> Change {
        self.0.statement.change.clone()
    }

    pub fn certificate(&self) -> &Certificate {
        &self.0.certificate
    }
}

impl ResolutionClaim {
    pub fn validate(
        &self,
        client: &Client,
        current_view: &View,
    ) -> Result<(), Top<ResolutionError>> {
        let view = client
            .view(&self.view)
            .ok_or(ResolutionError::UnknownView.into_top())
            .spot(here!())?;

        if view.height() > current_view.height() {
            return ResolutionError::FutureVote.fail().spot(here!());
        }

        current_view
            .validate_extension(&self.statement.change)
            .pot(ResolutionError::ViewError, here!())?;

        self.certificate
            .verify_quorum(&view, &self.statement)
            .pot(ResolutionError::CertificateInvalid, here!())?;

        Ok(())
    }

    pub fn to_resolution(
        self,
        client: &Client,
        current_view: &View,
    ) -> Result<Resolution, Top<ResolutionError>> {
        self.validate(client, current_view)?;
        Ok(Resolution(self))
    }
}

impl CryptoStatement for Statement {
    type Header = Header;
    const HEADER: Header = Header::Resolution;
}

impl From<Resolution> for ResolutionClaim {
    fn from(resolution: Resolution) -> Self {
        resolution.0
    }
}
