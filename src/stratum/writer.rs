use std::boxed::FnBox;
use std::io::{LineWriter, Write};

use super::*;

#[allow(dead_code)]
pub struct Writer {
    pub sender: Arc<Mutex<Sender<String>>>,
    handle: JoinHandle<()>,
    pub result: Receiver<Result<usize>>,
}

impl Writer {
    pub fn spawn(pool: &mut Pool) {
        let stream = pool.try_connect().unwrap().try_clone().unwrap();
        let mut linew = LineWriter::new(stream);
        let (data_tx, data_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut data = String::new();
            loop {
                let _ = result_tx.send(
                    Box::new(
                        |rx: &Receiver<String>| -> Result<usize> {
                            data = rx.recv().context("Writer recv err!")?;
                            dbg!(&data);
                            Ok(linew.write(data.as_bytes()).context("TcpSteam write err!")?)
                        }).call_box((&data_rx, ))
                );
            };
        });
        pool.writer = Some(Self {
            sender: Arc::new(Mutex::new(data_tx)),
            handle,
            result: result_rx,
        });
    }

    pub fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}
