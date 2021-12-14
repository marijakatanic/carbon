use crate::{
    commit::BatchCompletion, database::Database, discovery::Client,
    processing::processor::commit::errors::ServeCommitError,
};

use doomstack::{here, ResultExt, Top};

use talk::{net::Session, sync::voidable::Voidable};

pub(in crate::processing::processor::commit) async fn completion(
    discovery: &Client,
    database: &Voidable<Database>,
    session: Session,
    completion: BatchCompletion,
) -> Result<(), Top<ServeCommitError>> {
    // Validate `completion`

    completion
        .validate(discovery)
        .pot(ServeCommitError::BatchCompletionInvalid, here!())?;

    // Store `completion` in `completion.root()`'s `BatchHolder`, if still available in `database`

    {
        let mut database = database
            .lock()
            .pot(ServeCommitError::DatabaseVoid, here!())?;

        if let Some(holder) = database.commit.batches.get_mut(&completion.root()) {
            holder.attach(completion);
        }
    }

    session.end();

    Ok(())
}
