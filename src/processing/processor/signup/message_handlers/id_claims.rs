use crate::{
    crypto::Identify,
    database::Database,
    processing::{
        messages::SignupResponse, processor::signup::errors::ServeSignupError,
        processor_settings::Signup,
    },
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
    settings: &Signup,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    // Verify that `claims` is sorted and deduplicated

    if !claims
        .windows(2)
        .all(|window| window[0].client() < window[1].client())
    {
        return ServeSignupError::InvalidRequest.fail().spot(here!());
    }

    // Process `claims` into `shards`

    let mut transaction = CollectionTransaction::new();

    let shards = claims
        .into_iter()
        .map(|claim| {
            if claim.view() != view.identifier() {
                return ServeSignupError::ForeignView.fail().spot(here!());
            }

            claim
                .validate(settings.signup_settings.work_difficulty)
                .pot(ServeSignupError::InvalidRequest, here!())?;

            let stored = database
                .signup
                .claims
                .entry(claim.id())
                .or_insert(claim.clone());

            if stored.client() == claim.client() {
                // If `claim.id()` was already claimed by `claim.client()`, then
                // `claim.id()` will be inserted twice in `database.signup.claimed`
                // (which is harmless) and the `IdAssignment` will be repeated
                let _ = transaction.insert(claim.id());
                Ok(Ok(IdAssignment::certify(&keychain, &claim)))
            } else {
                // `claim.id()` was previously claimed by another client: return
                // the relevant `IdClaim` as proof of conflict
                Ok(Err(stored.clone()))
            }
        })
        .collect::<Result<Vec<Result<MultiSignature, IdClaim>>, Top<ServeSignupError>>>();

    // In order to keep `claims` in sync with `claimed`, `transaction`
    // must be executed before bailing (if `signatures` is `Err`)
    database.signup.claimed.execute(transaction);

    Ok(SignupResponse::IdAssignmentShards(shards?))
}
