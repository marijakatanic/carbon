use crate::{
    database::Database, discovery::Client, prepare::BatchCommit,
    processing::processor::prepare::errors::ServePrepareError,
};

use doomstack::{here, ResultExt, Top};

use talk::{net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::prepare) async fn commit(
    discovery: &Client,
    database: &Voidable<Database>,
    session: Session,
    commit: BatchCommit,
) -> Result<(), Top<ServePrepareError>> {
    // Validate `commit`

    commit
        .validate(discovery)
        .pot(ServePrepareError::InvalidCommit, here!())?;

    // Store `commit` in `commit.root()`'s `BatchHolder`, if still available in `database`

    {
        let mut database = database
            .lock()
            .pot(ServePrepareError::DatabaseVoid, here!())?;

        if let Some(holder) = database.prepare.batches.get_mut(&commit.root()) {
            holder.attach(commit);
        }
    }

    session.end();

    Ok(())
}
