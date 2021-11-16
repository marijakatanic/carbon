use crate::{
    churn::Churn,
    crypto::Identify,
    discovery::Client,
    lattice::{Element as LatticeElement, ElementError as LatticeElementError},
    view::{Increment, View},
    view_generator::ViewDecision,
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
    #[doom(description("`ViewProposal` is `Tail`, but `Install` message is tailless"))]
    InstallTailless,
    #[doom(description("`ViewProposal` contains an invalid `Churn`"))]
    InvalidChurn,
}

impl ViewProposal {
    pub(in crate::view_generator) fn to_decision(
        self,
        client: &Client,
        current_view: &View,
    ) -> ViewDecision {
        match self {
            ViewProposal::Churn { churn, .. } => {
                let churn: Increment = churn
                    .into_iter()
                    .map(|churn| churn.to_change(client, current_view).unwrap())
                    .collect();

                ViewDecision::Churn { churn }
            }
            ViewProposal::Tail { install } => ViewDecision::Tail { install },
        }
    }
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
        #[derive(Serialize)]
        #[repr(u8)]
        enum ProposalType {
            Churn = 0,
            Tail = 1,
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
