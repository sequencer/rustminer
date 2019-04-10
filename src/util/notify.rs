use std::sync::{Arc, Mutex};

use futures::task::{current, Task};
use futures::{Async::*, Future, Poll};

#[derive(Clone, Default)]
pub struct Notify {
    stat: Arc<Mutex<Option<()>>>,
    task: Arc<Mutex<Option<Task>>>,
}

impl Notify {
    pub fn notify(&self) {
        *self.stat.lock().unwrap() = Some(());
        if let Some(ref task) = *self.task.lock().unwrap() {
            task.notify()
        }
    }

    pub fn notified(&self) -> bool {
        self.stat.lock().unwrap().take().is_some()
    }
}

impl Future for Notify {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.stat.lock().unwrap().is_some() {
            Ok(Ready(()))
        } else {
            *self.task.lock().unwrap() = Some(current());
            Ok(NotReady)
        }
    }
}
