use crate::{
    crypto::Identify,
    discovery::Client,
    lattice::{Element as LatticeElement, ElementError as LatticeElementError},
    view::View,
    voting::ResolutionClaim,
};

use doomstack::{here, ResultExt, Top};

use serde::{Deserialize, Serialize};
use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum ViewProposal {
    New {
        install: Hash,
        resolutions: Vec<ResolutionClaim>,
    },

    Tail {
        install: Hash,
    },
}

impl LatticeElement for ViewProposal {
    fn validate(&self, client: &Client, view: &View) -> Result<(), Top<LatticeElementError>> {
        match &self {
            ViewProposal::New {
                install,
                resolutions,
            } => {
                for resolution in resolutions {
                    resolution
                        .validate(client, view)
                        .pot(LatticeElementError::ElementInvalid, here!())?;
                }
            }
            ViewProposal::Tail { install } => {}
        }

        Ok(())
    }
}

impl Identify for ViewProposal {
    fn identifier(&self) -> Hash {
        todo!()
    }
}
