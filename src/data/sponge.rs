use crate::data::SpongeSettings;

use std::{
    mem,
    sync::{Arc, Mutex},
    time::Instant,
};

use talk::sync::fuse::Fuse;

use tokio::{sync::Notify, time};

pub(crate) struct Sponge<Item> {
    database: Mutex<Database<Item>>,
    notify: Arc<Notify>,
    settings: SpongeSettings,
    fuse: Fuse,
}

struct Database<Item> {
    start: Instant,
    items: Vec<Item>,
}

impl<Item> Sponge<Item> {
    pub fn new(settings: SpongeSettings) -> Self {
        let database = Mutex::new(Database {
            start: Instant::now(),
            items: Vec::new(),
        });

        let notify = Arc::new(Notify::new());

        let fuse = Fuse::new();

        Sponge {
            database,
            notify,
            fuse,
            settings,
        }
    }

    pub fn push(&self, item: Item) {
        let mut database = self.database.lock().unwrap();

        database.items.push(item);

        if database.items.len() == 1 {
            database.start = Instant::now();

            let notify = self.notify.clone();
            let timeout = self.settings.timeout;

            self.fuse.spawn(async move {
                time::sleep(timeout).await;
                notify.notify_one();
            });
        }

        if database.items.len() >= self.settings.capacity {
            self.notify.notify_one();
        }
    }

    pub async fn flush(&self) -> Vec<Item> {
        loop {
            self.notify.notified().await;

            let mut database = self.database.lock().unwrap();

            if database.items.is_empty() {
                continue;
            }

            if database.items.len() >= self.settings.capacity
                || database.start.elapsed() > self.settings.timeout
            {
                let mut flush = Vec::new();
                mem::swap(&mut flush, &mut database.items);

                break flush;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    #[tokio::test]
    #[ignore]
    async fn empty() {
        let sponge = Arc::new(Sponge::<u32>::new(SpongeSettings {
            capacity: 10,
            timeout: Duration::from_secs_f64(0.1),
        }));

        {
            let sponge = sponge.clone();

            tokio::spawn(async move {
                sponge.flush().await;
                panic!("sponge flushed unexpectedly");
            });
        }

        time::sleep(Duration::from_secs(5)).await;
    }

    #[tokio::test]
    async fn timeout() {
        let sponge = Arc::new(Sponge::new(SpongeSettings {
            capacity: 10,
            timeout: Duration::from_secs_f64(0.1),
        }));

        let handle = {
            let sponge = sponge.clone();

            tokio::spawn(async move {
                let flush = sponge.flush().await;
                assert_eq!(flush.len(), 1);
            })
        };

        sponge.push(42u32);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn repeated_timeout() {
        let sponge = Arc::new(Sponge::new(SpongeSettings {
            capacity: 10,
            timeout: Duration::from_secs_f64(0.1),
        }));

        for size in 1..5 {
            let handle = {
                let sponge = sponge.clone();

                tokio::spawn(async move {
                    let flush = sponge.flush().await;
                    assert_eq!(flush.len(), size);
                })
            };

            for _ in 0..size {
                sponge.push(42u32);
            }

            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn overflow() {
        let sponge = Arc::new(Sponge::new(SpongeSettings {
            capacity: 10,
            timeout: Duration::from_secs_f64(0.5),
        }));

        let handle = {
            let sponge = sponge.clone();

            tokio::spawn(async move {
                let flush = sponge.flush().await;
                assert_eq!(flush.len(), 10);
            })
        };

        for _ in 0..10 {
            sponge.push(42u32);
            time::sleep(Duration::from_millis(1)).await;
        }

        handle.await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn repeated_overflow() {
        let sponge = Arc::new(Sponge::new(SpongeSettings {
            capacity: 10,
            timeout: Duration::from_secs_f64(0.5),
        }));

        {
            let sponge = sponge.clone();

            tokio::spawn(async move {
                loop {
                    let flush = sponge.flush().await;
                    assert!(flush.len() >= 10);
                }
            });
        }

        for _ in 0..1500 {
            sponge.push(42u32);
            time::sleep(Duration::from_millis(1)).await;
        }
    }
}
