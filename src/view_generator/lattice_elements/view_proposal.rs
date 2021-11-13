use crate::{
    churn::Churn,
    crypto::Identify,
    discovery::Client,
    lattice::{Element as LatticeElement, ElementError as LatticeElementError},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};
use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::{Hash, Hasher};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum ViewProposal {
    Churn { install: Hash, churn: Vec<Churn> },
    Tail { install: Hash },
}

#[derive(Doom)]
pub(crate) enum ViewProposalError {
    #[doom(description("Unknown `Install` message"))]
    InstallUnknown,
    #[doom(description("`Install` message does not reach the `View` provided"))]
    InvalidInstallDestination,
    #[doom(description("`ViewProposal` is `Churn`, but `Install` message has a tail"))]
    InstallTailed,
    #[doom(description("`ViewProposal` is `Churn`, but `Install` message is tailless"))]
    InstallTailless,
    #[doom(description("`ViewProposal` contains an invalid `Churn`"))]
    InvalidChurn,
}

impl LatticeElement for ViewProposal {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<LatticeElementError>> {
        match self {
            ViewProposal::Churn { install, churn } => {
                let install = client
                    .install(install)
                    .ok_or(ViewProposalError::InstallUnknown.into_top())
                    .pot(LatticeElementError::ElementInvalid, here!())?;

                let transition = install.into_transition();

                if transition.destination().identifier() != view.identifier() {
                    return ViewProposalError::InvalidInstallDestination
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }

                if !transition.tailless() {
                    return ViewProposalError::InstallTailed
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }

                for churn in churn.iter() {
                    churn
                        .validate(client, view)
                        .pot(ViewProposalError::InvalidChurn, here!())
                        .pot(LatticeElementError::ElementInvalid, here!())?;
                }
            }
            ViewProposal::Tail { install } => {
                let install = client
                    .install(install)
                    .ok_or(ViewProposalError::InstallUnknown.into_top())
                    .pot(LatticeElementError::ElementInvalid, here!())?;

                let transition = install.into_transition();

                if transition.destination().identifier() != view.identifier() {
                    return ViewProposalError::InvalidInstallDestination
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }

                if transition.tailless() {
                    return ViewProposalError::InstallTailless
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }
            }
        }

        Ok(())
    }
}

impl Identify for ViewProposal {
    fn identifier(&self) -> Hash {
        match self {
            ViewProposal::Churn { install, churn } => {
                let mut hasher = Hasher::new();

                for churn in churn.iter() {
                    hasher.update(&churn.identifier()).unwrap();
                }

                let churn = hasher.finalize();

                hash::hash(&(install, churn)).unwrap()
            }
            ViewProposal::Tail { install } => *install,
        }
    }
}
