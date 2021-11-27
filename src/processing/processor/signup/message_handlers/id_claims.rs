use crate::{
    crypto::Identify,
    database::Database,
    processing::{messages::SignupResponse, processor::signup::errors::ServeSignupError},
    signup::{IdAssignment, IdClaim},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::crypto::{primitives::multi::Signature as MultiSignature, KeyChain};

use zebra::database::CollectionTransaction;

pub(in crate::processing::processor::signup) fn id_claims(
    keychain: &KeyChain,
    view: &View,
    database: &mut Database,
    claims: Vec<IdClaim>,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    let mut transaction = CollectionTransaction::new();

    let signatures = claims
        .into_iter()
        .map(|claim| {
            if claim.view() != view.identifier() {
                return ServeSignupError::ForeignView.fail().spot(here!());
            }

            claim
                .validate()
                .pot(ServeSignupError::InvalidRequest, here!())?;

            let stored = database
                .signup
                .claims
                .entry(claim.id())
                .or_insert(claim.clone());

            if stored.client() == claim.client() {
                // Double-inserts are harmless
                let _ = transaction.insert(claim.id());
                Ok(Ok(IdAssignment::certify(&keychain, &claim)))
            } else {
                Ok(Err(stored.clone())) // Already claimed by another identity
            }
        })
        .collect::<Result<Vec<Result<MultiSignature, IdClaim>>, Top<ServeSignupError>>>();

    // In order to keep `claims` in sync with `claimed`, `transaction` is
    // executed before bailing (if `signatures` is `Err`)
    database.signup.claimed.execute(transaction);
    Ok(SignupResponse::IdAssignments(signatures?))
}
