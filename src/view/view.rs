use crate::view::{Change, CHANGES, FAMILY};

use std::sync::Arc;

use talk::crypto::primitives::sign::PublicKey;

use zebra::database::{Collection, CollectionTransaction};
use zebra::Commitment;

pub(crate) struct View {
    identifier: Commitment,
    data: Arc<Data>,
}

struct Data {
    changes: Collection<Change>,
    members: Vec<PublicKey>,
}

impl View {
    pub async fn genesis<M>(members: M) -> Self
    where
        M: IntoIterator<Item = PublicKey>,
    {
        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort();

        if members.len() < 4 {
            panic!(
                "called `genesis` with insufficient `members` for Byzantine resilience (i.e., 4)"
            );
        }

        let changes = members.iter().map(|replica| Change::Join(*replica));

        let mut collection = FAMILY.empty_collection();
        let mut transaction = CollectionTransaction::new();

        for change in changes {
            transaction
                .insert(change)
                .expect("called `genesis` with non-distinct `members`");
        }

        collection.execute(transaction).await;

        let identifier = collection.commit();

        CHANGES
            .lock()
            .unwrap()
            .insert(identifier, collection.clone());

        let data = Arc::new(Data {
            changes: collection,
            members: members,
        });

        View { identifier, data }
    }
}
