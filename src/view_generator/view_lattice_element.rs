use crate::{
    churn::Churn,
    crypto::Identify,
    discovery::Client,
    lattice::{Element as LatticeElement, ElementError as LatticeElementError},
    view::{Increment, View},
    view_generator::ViewLatticeBrief,
};

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use talk::crypto::primitives::{hash, hash::Hash};

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) enum ViewLatticeElement {
    Churn {
        install: Hash,
        churn: BTreeSet<Churn>,
    },
    Tail {
        install: Hash,
    },
}

#[derive(Doom)]
pub(crate) enum ViewLatticeElementError {
    #[doom(description("Unknown `Install` message"))]
    InstallUnknown,
    #[doom(description("`Install` message does not reach the `View` provided"))]
    InvalidInstallDestination,
    #[doom(description("`ViewProposal` is `Churn`, but `Install` message is not tailless"))]
    InstallNotTailless,
    #[doom(description("`ViewProposal` is `Tail`, but `Install` message is not tailed"))]
    InstallNotTailed,
    #[doom(description("`ViewProposal` contains an invalid `Churn`"))]
    InvalidChurn,
}

impl ViewLatticeElement {
    pub(in crate::view_generator) fn to_brief(
        self,
        client: &Client,
        view: &View,
    ) -> ViewLatticeBrief {
        match self {
            ViewLatticeElement::Churn { churn, .. } => {
                let churn: Increment = churn
                    .into_iter()
                    .map(|churn| churn.to_change(client, view).unwrap())
                    .collect();

                ViewLatticeBrief::Churn { churn }
            }
            ViewLatticeElement::Tail { install } => ViewLatticeBrief::Tail { install },
        }
    }
}

impl LatticeElement for ViewLatticeElement {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<LatticeElementError>> {
        match self {
            ViewLatticeElement::Churn { install, churn } => {
                let install = client
                    .install(install)
                    .ok_or(ViewLatticeElementError::InstallUnknown.into_top())
                    .pot(LatticeElementError::ElementInvalid, here!())?;

                let transition = install.into_transition();

                if transition.destination().identifier() != view.identifier() {
                    return ViewLatticeElementError::InvalidInstallDestination
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }

                if !transition.tailless() {
                    return ViewLatticeElementError::InstallNotTailless
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }

                for churn in churn.iter() {
                    churn
                        .validate(client, view)
                        .pot(ViewLatticeElementError::InvalidChurn, here!())
                        .pot(LatticeElementError::ElementInvalid, here!())?;
                }
            }
            ViewLatticeElement::Tail { install } => {
                let install = client
                    .install(install)
                    .ok_or(ViewLatticeElementError::InstallUnknown.into_top())
                    .pot(LatticeElementError::ElementInvalid, here!())?;

                let transition = install.into_transition();

                if transition.destination().identifier() != view.identifier() {
                    return ViewLatticeElementError::InvalidInstallDestination
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }

                if transition.tailless() {
                    return ViewLatticeElementError::InstallNotTailed
                        .fail()
                        .pot(LatticeElementError::ElementInvalid, here!());
                }
            }
        }

        Ok(())
    }
}

impl Identify for ViewLatticeElement {
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
            ViewLatticeElement::Churn { churn, .. } => {
                (ProposalType::Churn.identifier(), churn.identifier()).identifier()
            }
            ViewLatticeElement::Tail { install } => {
                (ProposalType::Tail.identifier(), install.identifier()).identifier()
            }
        }
    }
}
