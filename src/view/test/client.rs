use crate::view::{Install, View};

pub(crate) struct Client {
    current: View,
    last_installable: View,
}

impl Client {
    pub(crate) fn new(current: View, last_installable: View) -> Self {
        Self {
            current,          // The client's current view
            last_installable, // Only installable views *that the remote has knowledge about*
        }
    }

    pub(crate) async fn update(&mut self, installs: Vec<Install>) {
        let mut current = self.last_installable.clone();

        for install in installs {
            assert_eq!(current.identifier(), install.source());
            assert!(install.increments().len() > 0);

            let increment = install.increments()[0].clone();
            current = current.extend(increment).await;

            if install.increments().len() == 1 {
                // `install` is tailless
                self.last_installable = current.clone();
            }
        }

        if self.current.height() < current.height() {
            self.current = current;
        }
    }

    pub(crate) fn current(&self) -> &View {
        &self.current
    }

    pub(crate) fn last_installable(&self) -> &View {
        &self.last_installable
    }
}
