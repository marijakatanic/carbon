use crate::{
    database::Database,
    discovery::Client,
    prepare::{ReductionStatement, SignedBatch, WitnessStatement},
    processing::processor::prepare::{errors::ServePrepareError, steps},
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{
    crypto::{primitives::multi::Signature as MultiSignature, KeyChain},
    net::Session,
    sync::voidable::Voidable,
};

pub(in crate::processing::processor::prepare) async fn validate_signed(
    keychain: &KeyChain,
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    batch: &SignedBatch,
) -> Result<MultiSignature, Top<ServePrepareError>> {
    // Verify `batch.prepares()` is strictly increasing by `Id`
    // (this ensures searchability and non-duplication of `Id`s)
    if !batch
        .prepares()
        .windows(2)
        .all(|window| window[0].id() < window[1].id())
    {
        return ServePrepareError::MalformedBatch.fail().spot(here!());
    }

    // Identify unknown `Id`s in `batch`. If any, retrieve and store
    // all missing `IdAssignment`s

    steps::gather_assignments(discovery, database, session, batch).await?;

    {
        let database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        let mut reduction_signers = Vec::new();

        for (prepare, individual_signature) in
            batch.prepares().iter().zip(batch.individual_signatures())
        {
            let keycard = database.assignments[&prepare.id()].keycard();

            match individual_signature {
                Some(signature) => {
                    signature
                        .verify(&keycard, prepare)
                        .pot(ServePrepareError::InvalidBatch, here!())?;
                }
                None => {
                    reduction_signers.push(keycard);
                }
            }
        }

        let reduction_statement = ReductionStatement::new(batch.root());

        batch
            .reduction_signature()
            .verify(reduction_signers, &reduction_statement)
            .pot(ServePrepareError::InvalidBatch, here!())?;
    }

    let witness_statement = WitnessStatement::new(batch.root());
    let witness_shard = keychain.multisign(&witness_statement).unwrap();

    Ok(witness_shard)
}
