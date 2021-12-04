use bit_vec::BitVec;

use crate::view::View;

use doomstack::{here, Doom, ResultExt, Top};

use serde::{Deserialize, Serialize};

use talk::crypto::{primitives::multi::Signature as MultiSignature, Identity, Statement};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Certificate {
    signers: BitVec,
    signature: MultiSignature,
}

#[derive(Doom)]
pub(crate) enum CertificateError {
    #[doom(description("Certificate invalid"))]
    CertificateInvalid,
    #[doom(description("Not enough signers"))]
    NotEnoughSigners,
    #[doom(description("Overlapping signers"))]
    OverlappingSigners,
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
        for (index, member) in view.members().keys().enumerate() {
            if signer_ids.peek() == Some(&member) {
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

    pub fn aggregate_plurality<C>(view: &View, components: C) -> Self
    where
        C: IntoIterator<Item = (Identity, MultiSignature)>,
    {
        let certificate = Self::aggregate(view, components);

        #[cfg(debug_assertions)]
        {
            if certificate.power() < view.plurality() {
                panic!("Called `Certificate::aggregate` with an insufficient number of signers for a plurality");
            }
        }

        certificate
    }

    pub fn aggregate_quorum<C>(view: &View, components: C) -> Self
    where
        C: IntoIterator<Item = (Identity, MultiSignature)>,
    {
        let certificate = Self::aggregate(view, components);

        #[cfg(debug_assertions)]
        {
            if certificate.power() < view.quorum() {
                panic!("Called `Certificate::aggregate` with an insufficient number of signers for a quorum");
            }
        }

        certificate
    }

    pub fn power(&self) -> usize {
        self.signers.iter().filter(|mask| *mask).count()
    }

    pub fn verify_raw<S>(&self, view: &View, message: &S) -> Result<(), Top<CertificateError>>
    where
        S: Statement,
    {
        self.signature
            .verify(
                view.members()
                    .values()
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
            .pot(CertificateError::CertificateInvalid, here!())
    }

    pub fn verify_threshold<S>(
        &self,
        view: &View,
        message: &S,
        threshold: usize,
    ) -> Result<(), Top<CertificateError>>
    where
        S: Statement,
    {
        if self.power() >= threshold {
            self.verify_raw(view, message)
        } else {
            CertificateError::NotEnoughSigners.fail()
        }
    }

    pub fn verify_plurality<S>(&self, view: &View, message: &S) -> Result<(), Top<CertificateError>>
    where
        S: Statement,
    {
        self.verify_threshold(view, message, view.plurality())
    }

    pub fn verify_quorum<S>(&self, view: &View, message: &S) -> Result<(), Top<CertificateError>>
    where
        S: Statement,
    {
        self.verify_threshold(view, message, view.quorum())
    }

    pub fn distinct_power<'c, C>(certificates: C) -> Result<usize, Top<CertificateError>>
    where
        C: IntoIterator<Item = &'c Certificate>,
    {
        let mut certificates = certificates.into_iter().peekable();

        let members = match certificates.peek() {
            Some(first) => first.signers.len(),
            None => return Ok(0),
        };

        let mut cover = vec![false; members];

        for certificate in certificates {
            if certificate.signers.len() != members {
                panic!("called `Certificate::coverage` with `Certificate`s from different views");
            }

            cover = cover
                .into_iter()
                .zip(certificate.signers.iter())
                .map(|(lho, rho)| {
                    if lho && rho {
                        CertificateError::OverlappingSigners.fail().spot(here!())
                    } else {
                        Ok(lho || rho)
                    }
                })
                .collect::<Result<Vec<bool>, Top<CertificateError>>>()?;
        }

        Ok(cover.into_iter().filter(|mask| *mask).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Certificate {
        pub fn new(signers: BitVec, signature: MultiSignature) -> Self {
            Certificate { signers, signature }
        }
    }
}
