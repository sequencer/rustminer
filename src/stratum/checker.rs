use std::time::Duration;

use tokio::timer::Interval;

use super::*;

pub fn new(pool: &mut Pool) -> impl Future<Item = (), Error = ()> + Send {
    let last_active = pool.last_active.clone();
    let timeout = Duration::from_secs(60);
    Interval::new_interval(Duration::from_secs(5))
        .for_each(move |x| {
            match *last_active.lock().unwrap() {
                Ok(t) if t + timeout >= x => (),
                _ => {
                    eprintln!("pool connect timeout!");
                }
            }
            Ok(())
        })
        .map_err(|_| ())
}
