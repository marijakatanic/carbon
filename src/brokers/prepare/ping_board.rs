use crate::view::View;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use talk::crypto::Identity;

pub(in crate::brokers::prepare) struct PingBoard(Arc<Mutex<HashMap<Identity, Duration>>>);

impl PingBoard {
    pub fn new(view: &View) -> Self {
        let board = view
            .members()
            .keys()
            .copied()
            .map(|replica| (replica, Duration::MAX))
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
