use crate::{
    crypto::Identify,
    database::Database,
    discovery::Client,
    prepare::{SignedBatch, WitnessedBatch},
    processing::{
        messages::PrepareRequest,
        processor::prepare::{errors::ServePrepareError, steps},
    },
    view::View,
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::KeyChain, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn witnessed_batch(
    keychain: &KeyChain,
    discovery: &Client,
    view: &View,
    database: &Voidable<Database>,
    session: &mut Session,
) -> Result<WitnessedBatch, Top<ServePrepareError>> {
    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let prepares = match request {
        PrepareRequest::Prepares(batch) => batch,
        _ => return ServePrepareError::UnexpectedRequest.fail().spot(here!()),
    };

    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let batch = match request {
        PrepareRequest::Witness(witness) => {
            Ok(WitnessedBatch::new(view.identifier(), prepares, witness))
        }
        PrepareRequest::Signatures(reduction_signature, individual_signatures) => {
            let batch = SignedBatch::new(prepares, reduction_signature, individual_signatures);

            let witness_shard =
                steps::validate_signed(keychain, discovery, database, session, &batch).await?;

            let witness = steps::trade_witnesses(view, session, &batch, witness_shard).await?;

            let batch = batch.into_witnessed(view.identifier(), witness);

            Ok(batch)
        }
        _ => ServePrepareError::UnexpectedRequest.fail().spot(here!()),
    }?;

    batch
        .validate(discovery)
        .pot(ServePrepareError::InvalidBatch, here!())?;

    Ok(batch)
}
