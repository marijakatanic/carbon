use crate::view::{Change, Increment, FAMILY, VIEWS};

use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::sync::Arc;

use talk::crypto::KeyCard;

use zebra::database::{Collection, CollectionTransaction};
use zebra::Commitment;

#[derive(Clone)]
pub(crate) struct View {
    data: Arc<Data>,
}

struct Data {
    changes: Collection<Change>,
    members: Vec<KeyCard>,
}

impl View {
    pub async fn genesis<M>(members: M) -> Self
    where
        M: IntoIterator<Item = KeyCard>,
    {
        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort_by_key(KeyCard::identity);

        #[cfg(debug_assertions)]
        {
            // Verify that all `members` are distinct, and sufficiently many for Byzantine resilience.

            let members_set = members.clone().into_iter().collect::<HashSet<_>>();

            if members_set.len() > members.len() {
                panic!("called `View::genesis` with non-distinct `members`");
            }

            if members.len() < 4 {
                panic!("called `View::genesis` with insufficient `members` for Byzantine resilience (i.e., 4)");
            }
        }

        let updates = members
            .clone()
            .into_iter()
            .map(|replica| Change::Join(replica));

        let mut changes = FAMILY.empty_collection();
        let mut transaction = CollectionTransaction::new();

        for update in updates {
            transaction.insert(update).unwrap();
        }

        changes.execute(transaction).await;

        let identifier = changes.commit();

        let data = Arc::new(Data { changes, members });
        let view = View { data };

        match VIEWS.lock().unwrap().entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(view).clone(),
        }
    }

    pub async fn extend(&self, increment: Increment) -> Self {
        let updates = increment.into_vec();

        #[cfg(debug_assertions)]
        {
            // Verify that no element of `updates` is already in `self.data.changes`,
            // and that all negative changes of `updates` are matched by a corresponding
            // positive change in `self.data.changes`.
            //
            // (Note that all identities in `updates` are already guaranteed to be distinct)

            use std::collections::HashMap;

            let requirements = updates
                .iter()
                .filter_map(Change::requirement)
                .collect::<Vec<_>>();

            // These are all guaranteed to be distinct: indeed, all identities in `updates`
            // are distinct, and `requirements` is a mirror of a subsequence in `updates`
            // (`Change::Leave`s are mapped onto corresponding `Change::Join`s).
            let queries = updates.clone().into_iter().chain(requirements.into_iter());

            let mut transaction = CollectionTransaction::new();

            let queries = queries
                .map(|change| {
                    let query = transaction.contains(&change).unwrap();
                    (change, query)
                })
                .collect::<Vec<_>>();

            let response = self.data.changes.clone().execute(transaction).await;

            let response = queries
                .into_iter()
                .map(|(change, query)| (change, response.contains(&query)))
                .collect::<HashMap<_, _>>();

            for update in updates.iter() {
                // Verify that `update` is not already in `self.data.changes`
                if response[update] {
                    panic!("called `View::extend` with a pre-existing `Change`");
                }

                // Verify that, if `update` is negative, its positive mirror is in `self.data.changes`
                if let Some(requirement) = update.requirement() {
                    if !response[&requirement] {
                        panic!("called `View::extend` with an unsatisfied requirement (unmatched `Change::Leave`)");
                    }
                }
            }
        }

        let mut changes = self.data.changes.clone();
        let mut transaction = CollectionTransaction::new();

        for update in updates.clone() {
            transaction.insert(update).unwrap();
        }

        changes.execute(transaction).await;

        let identifier = changes.commit();

        let mut members = self
            .data
            .members
            .clone()
            .into_iter()
            .collect::<HashSet<_>>();

        for update in updates {
            match update {
                Change::Join(replica) => {
                    members.insert(replica);
                }
                Change::Leave(replica) => {
                    members.remove(&replica);
                }
            }
        }

        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort_by_key(KeyCard::identity);

        let data = Arc::new(Data { changes, members });
        let view = View { data };

        match VIEWS.lock().unwrap().entry(identifier) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry.insert(view).clone(),
        }
    }

    pub fn get(identifier: Commitment) -> Option<Self> {
        VIEWS.lock().unwrap().get(&identifier).cloned()
    }

    pub fn identifier(&self) -> Commitment {
        self.data.changes.commit()
    }

    pub fn plurality(&self) -> usize {
        (self.data.members.len() - 1) / 3 + 1
    }

    pub fn quorum(&self) -> usize {
        self.data.members.len() - (self.data.members.len() - 1) / 3
    }

    pub fn members(&self) -> &[KeyCard] {
        self.data.members.as_slice()
    }
}
