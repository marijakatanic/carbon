use crate::{
    churn::Churn,
    crypto::Identify,
    discovery::Client,
    lattice::{Element as LatticeElement, ElementError as LatticeElementError},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum ViewProposal {
    Churn {
        install: Hash,
        churn: BTreeSet<Churn>,
    },
    Tail {
        install: Hash,
    },
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

// TODO: Determine if this implementation could be made
// more relaxed: for example, all tailless `Install`
// messages have the same effect (i.e., enabling the
// application of `Churn`s)
impl Identify for ViewProposal {
    fn identifier(&self) -> Hash {
        #[derive(Serialize)]
        #[repr(u8)]
        enum ProposalType {
            Churn,
            Tail,
        }

        impl Identify for ProposalType {
            fn identifier(&self) -> Hash {
                hash::hash(&self).unwrap()
            }
        }

        match self {
            ViewProposal::Churn { churn, .. } => {
                (ProposalType::Churn.identifier(), churn.identifier()).identifier()
            }
            ViewProposal::Tail { install } => {
                (ProposalType::Tail.identifier(), install.identifier()).identifier()
            }
        }
    }
}
