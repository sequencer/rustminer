use std::time::Duration;

use futures::future::{loop_fn, Loop};
use tokio::timer::Delay;

use super::*;

impl Pool {
    pub fn checker(&mut self) -> impl Future<Item = (), Error = ()> + Send {
        let interval = Duration::from_secs(1);
        let timeout = Duration::from_secs(60);

        loop_fn(self.last_active.clone(), move |last_active| {
            let start = Instant::now();
            trace!("checker delay {:?}: {:?}", interval, start);

            Delay::new(start + interval)
                .map_err(|e| error!("checker delay err: {:?}", e))
                .and_then(move |_| {
                    let now = Instant::now();
                    trace!("checker run: {:?}", now);

                    if now > *last_active.lock().unwrap() + timeout {
                        error!("pool connection timeout!");
                        Ok(Loop::Break(()))
                    } else {
                        Ok(Loop::Continue(last_active))
                    }
                })
        })
    }
}
