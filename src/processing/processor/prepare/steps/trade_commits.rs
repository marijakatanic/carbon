use crate::{
    database::Database,
    discovery::Client,
    prepare::BatchCommitShard,
    processing::{
        messages::{PrepareRequest, PrepareResponse},
        processor::prepare::errors::ServePrepareError,
    },
};

use doomstack::{here, Doom, ResultExt, Top};

use talk::{crypto::primitives::hash::Hash, net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn trade_commits(
    discovery: &Client,
    database: &Voidable<Database>,
    session: &mut Session,
    root: Hash,
    shard: BatchCommitShard,
) -> Result<(), Top<ServePrepareError>> {
    session
        .send(&PrepareResponse::CommitShard(shard))
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let request = session
        .receive::<PrepareRequest>()
        .await
        .pot(ServePrepareError::ConnectionError, here!())?;

    let commit = match request {
        PrepareRequest::Commit(commit) => commit,
        _ => return ServePrepareError::UnexpectedRequest.fail().spot(here!()),
    };

    if commit.root() != root {
        return ServePrepareError::ForeignCommit.fail().spot(here!());
    }

    commit
        .validate(discovery)
        .pot(ServePrepareError::InvalidCommit, here!())?;

    let mut database = database
        .lock()
        .pot(ServePrepareError::DatabaseVoid, here!())?;

    if let Some(holder) = database.prepare.batches.get_mut(&root) {
        holder.commit(commit);
    }

    Ok(())
}
