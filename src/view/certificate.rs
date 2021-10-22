use bit_vec::BitVec;

use crate::view::View;

use doomstack::Top;

use talk::crypto::primitives::multi::{MultiError, Signature as MultiSignature};
use talk::crypto::primitives::sign::PublicKey;
use talk::crypto::Statement;

pub(crate) struct Certificate {
    signers: BitVec,
    signature: MultiSignature,
}

impl Certificate {
    pub fn aggregate<C>(view: &View, components: C) -> Self
    where
        C: IntoIterator<Item = (PublicKey, MultiSignature)>,
    {
        let mut components = components.into_iter().collect::<Vec<_>>();
        components.sort_by_key(|component| component.0);

        let mut signers = BitVec::from_elem(view.members().len(), false);
        let mut members = view.members().iter().enumerate();

        for (replica, _) in components.iter() {
            let (index, member) = members
                .next()
                .expect("Called `Certificate::aggregate` with a foreign component");

            if *replica == member.root() {
                signers.set(index, true);
            }
        }

        let signatures = components.into_iter().map(|component| component.1);

        let signature = MultiSignature::aggregate(signatures)
            .expect("Called `Certificate::aggregate` with an incorrect multi-signature");

        Certificate { signers, signature }
    }

    pub fn verify<S>(&self, view: &View, message: &S) -> Result<(), Top<MultiError>>
    where
        S: Statement,
    {
        self.signature.verify(
            view.members()
                .iter()
                .enumerate()
                .filter_map(|(index, card)| {
                    if self.signers[index] {
                        Some(card)
                    } else {
                        None
                    }
                }),
            message,
        )
    }
}
