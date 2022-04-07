use crate::view::View;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use talk::crypto::Identity;

#[derive(Clone)]
pub(crate) struct PingBoard(Arc<Mutex<HashMap<Identity, Duration>>>);

impl PingBoard {
    pub fn new(view: &View) -> Self {
        let board = view
            .members()
            .keys()
            .copied()
            .map(|member| (member, Duration::MAX))
            .collect::<HashMap<_, _>>();

        let board = Arc::new(Mutex::new(board));

        PingBoard(board)
    }

    pub fn submit(&self, replica: Identity, ping: Duration) {
        let mut board = self.0.lock().unwrap();
        board.insert(replica, ping);
    }

    pub fn rankings(&self) -> Vec<Identity> {
        let board = self.0.lock().unwrap();

        let mut pings = board
            .iter()
            .map(|(replica, ping)| (*replica, *ping))
            .collect::<Vec<_>>();

        pings.sort_by_key(|(_, ping)| *ping);

        pings.into_iter().map(|(replica, _)| replica).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::view::test::InstallGenerator;

    #[test]
    fn manual() {
        let generator = InstallGenerator::new(4);

        let view = generator.view(4);
        let identities = view.members().keys().copied().collect::<Vec<_>>();

        let board = PingBoard::new(&view);

        board.submit(identities[2], Duration::from_secs(3));
        board.submit(identities[0], Duration::from_secs(1));
        board.submit(identities[1], Duration::from_secs(2));

        let rankings = board.rankings();
        assert_eq!(rankings, identities);
    }
}
