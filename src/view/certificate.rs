use bit_vec::BitVec;

use crate::view::View;

use doomstack::Top;

use talk::crypto::primitives::multi::{MultiError, Signature as MultiSignature};
use talk::crypto::{Identity, Statement};

pub(crate) struct Certificate {
    signers: BitVec,
    signature: MultiSignature,
}

impl Certificate {
    pub fn aggregate<C>(view: &View, components: C) -> Self
    where
        C: IntoIterator<Item = (Identity, MultiSignature)>,
    {
        let mut components = components.into_iter().collect::<Vec<_>>();
        components.sort_by_key(|component| component.0);

        let mut signers = BitVec::from_elem(view.members().len(), false);
        let mut signer_ids = components.iter().map(|component| component.0).peekable();

        // Both `view.members()` and `signer_ids` are sorted. In order to determine which
        // elements of `signers` to set to `true`, loop thorugh all elements of `view.members()`:
        // for every `member`, if `member` is the next element of `signer_ids`, then set the
        // corresponding element of `signers` to `true`, and move `signer_ids` on.
        for (index, member) in view.members().iter().enumerate() {
            if signer_ids.peek() == Some(&member.identity()) {
                signers.set(index, true);
                signer_ids.next().unwrap();
            }
        }

        if signer_ids.next().is_some() {
            panic!("Called `Certificate::aggregate` with a foreign component");
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
