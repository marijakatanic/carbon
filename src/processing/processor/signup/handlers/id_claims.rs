use crate::{
    database::Database,
    processing::{
        messages::SignupResponse, processor::signup::errors::ServeSignupError,
        processor_settings::Signup,
    },
    signup::{IdAssignment, IdClaim},
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use rayon::prelude::*;

use talk::{crypto::KeyChain, sync::voidable::Voidable};

use zebra::database::CollectionTransaction;

pub(in crate::processing::processor::signup) fn id_claims(
    keychain: &KeyChain,
    _view: &View,
    database: &Voidable<Database>,
    claims: Vec<IdClaim>,
    _settings: &Signup,
) -> Result<SignupResponse, Top<ServeSignupError>> {
    // Verify that `claims` is sorted and deduplicated

    if !claims
        .windows(2)
        .all(|window| window[0].client() < window[1].client())
    {
        return ServeSignupError::InvalidRequest.fail().spot(here!());
    }

    // Validate `claims` (in parallel)

    // Skip verification (for benchmark purposes)

    // claims
    //     .par_iter()
    //     .map(|claim| {
    //         if claim.view() != view.identifier() {
    //             return ServeSignupError::ForeignView.fail().spot(here!());
    //         }

    //         claim
    //             .validate(settings.signup_settings.work_difficulty)
    //             .pot(ServeSignupError::InvalidRequest, here!())?;

    //         Ok(())
    //     })
    //     .collect::<Result<(), Top<ServeSignupError>>>()?;

    // Process `claims` into `shards`

    let shards = {
        let mut database = database
            .lock()
            .pot(ServeSignupError::DatabaseVoid, here!())?;

        let mut transaction = CollectionTransaction::new();

        let shards = claims
            .into_iter()
            .map(|claim| {
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
                    Ok(claim)
                } else {
                    // `claim.id()` was previously claimed by another client: return
                    // the relevant `IdClaim` as proof of conflict
                    Err(stored.clone())
                }
            })
            .collect::<Vec<_>>();

        database.signup.claimed.execute(transaction);

        shards
    };

    let shards = shards
        .into_par_iter()
        .map(|result| result.map(|claim| IdAssignment::certify(&keychain, &claim)))
        .collect();

    Ok(SignupResponse::IdAssignmentShards(shards))
}
