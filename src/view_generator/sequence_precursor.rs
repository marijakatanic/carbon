use crate::{crypto::Identify, view::Increment};

use serde::{Deserialize, Serialize};

use talk::crypto::primitives::hash;
use talk::crypto::primitives::hash::Hash;

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::view_generator) enum SequencePrecursor {
    Churn { churn: Increment },
    Tail { install: Hash },
}

impl Identify for SequencePrecursor {
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
            SequencePrecursor::Churn { churn, .. } => {
                (ProposalType::Churn.identifier(), churn.identifier()).identifier()
            }
            SequencePrecursor::Tail { install } => {
                (ProposalType::Tail.identifier(), install.identifier()).identifier()
            }
        }
    }
}
