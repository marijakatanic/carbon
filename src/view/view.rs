use crate::{
    directory::Directory,
    view::{Change, CHANGES, FAMILY, MEMBERS},
};

use futures::stream::{FuturesOrdered, StreamExt};

use std::collections::HashSet;
use std::sync::Arc;

use talk::crypto::{Identity, KeyCard};

use zebra::database::{Collection, CollectionTransaction};
use zebra::Commitment;

pub(crate) struct View {
    data: Arc<Data>,
}

struct Data {
    changes: Collection<Change>,
    members: Vec<KeyCard>,
}

impl View {
    pub async fn genesis<M>(members: M, directory: &Directory) -> Self
    where
        M: IntoIterator<Item = Identity>,
    {
        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort();

        #[cfg(debug_assertions)]
        {
            let members_set = members.clone().into_iter().collect::<HashSet<_>>();

            if members_set.len() > members.len() {
                panic!("called `View::genesis` with non-distinct `members`");
            }

            if members.len() < 4 {
                panic!("called `View::genesis` with insufficient `members` for Byzantine resilience (i.e., 4)");
            }
        }

        let updates = members.iter().map(|replica| Change::Join(*replica));

        let mut changes = FAMILY.empty_collection();
        let mut transaction = CollectionTransaction::new();

        for change in updates {
            transaction.insert(change).unwrap();
        }

        changes.execute(transaction).await;

        let identifier = changes.commit();

        let members = members
            .into_iter()
            .map(|member| directory.get_card(member))
            .collect::<FuturesOrdered<_>>()
            .map(|member| {
                member.expect("called `View::genesis` on a member with unknown `KeyCard`")
            })
            .collect::<Vec<_>>()
            .await;

        CHANGES.lock().unwrap().insert(identifier, changes.clone());
        MEMBERS.lock().unwrap().insert(identifier, members.clone());

        let data = Arc::new(Data { changes, members });

        View { data }
    }

    pub async fn extend<C>(&self, updates: C, directory: &Directory) -> Self
    where
        C: IntoIterator<Item = Change>,
    {
        let updates = updates.into_iter().collect::<Vec<_>>();

        #[cfg(debug_assertions)]
        {
            use std::collections::HashMap;

            let updates_set = updates.clone().into_iter().collect::<HashSet<_>>();

            if updates_set.len() > updates.len() {
                panic!("called `View::extend` with non-distinct `updates`");
            }

            let positive_updates = updates.iter().cloned().filter(Change::is_join);
            let negative_updates = updates.iter().cloned().filter(Change::is_leave);

            let matching_updates = updates
                .iter()
                .cloned()
                .filter(Change::is_leave)
                .map(Change::mirror);

            let queries = positive_updates
                .chain(negative_updates)
                .chain(matching_updates)
                .collect::<HashSet<_>>();

            let mut transaction = CollectionTransaction::new();

            let queries = queries
                .into_iter()
                .map(|change| (change, transaction.contains(&change).unwrap()))
                .collect::<Vec<_>>();

            let response = self.data.changes.clone().execute(transaction).await;

            let response = queries
                .into_iter()
                .map(|(change, query)| (change, response.contains(&query)))
                .collect::<HashMap<_, _>>();

            for update in updates.iter() {
                if response[update] {
                    panic!("called `View::extend` with a pre-existing `Change`");
                }

                if update.is_leave() && !response[&update.mirror()] {
                    panic!("called `View::extend` with an unmatched `Change::Leave`");
                }
            }
        }

        let mut changes = self.data.changes.clone();
        let mut transaction = CollectionTransaction::new();

        for update in updates.iter() {
            transaction.insert(*update).unwrap();
        }

        changes.execute(transaction).await;

        let identifier = changes.commit();

        let mut members = self
            .data
            .members
            .iter()
            .map(KeyCard::identity)
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
        members.sort();

        let members = members
            .into_iter()
            .map(|member| directory.get_card(member))
            .collect::<FuturesOrdered<_>>()
            .map(|member| {
                member.expect("called `View::genesis` on a member with unknown `KeyCard`")
            })
            .collect::<Vec<_>>()
            .await;

        CHANGES.lock().unwrap().insert(identifier, changes.clone());
        MEMBERS.lock().unwrap().insert(identifier, members.clone());

        let data = Arc::new(Data { changes, members });

        View { data }
    }

    pub fn get(identifier: Commitment) -> Option<Self> {
        let changes = CHANGES.lock().unwrap().get(&identifier).cloned();
        let members = MEMBERS.lock().unwrap().get(&identifier).cloned();

        match (changes, members) {
            (Some(changes), Some(members)) => {
                let data = Arc::new(Data { changes, members });
                Some(View { data })
            }
            _ => None,
        }
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
