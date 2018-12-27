use std::io::Write;
use std::boxed::FnBox;

use super::*;

#[allow(dead_code)]
pub struct Writer {
    pub sender: Sender<String>,
    handle: JoinHandle<()>,
    pub result: Receiver<Result<usize>>,
}

impl Writer {
    pub fn new(stream: &TcpStream) -> Self {
        let mut stream = stream.try_clone().unwrap();
        let (data_tx, data_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut data = String::new();
            loop {
                let _ = result_tx.send(
                    Box::new(
                        |rx: &Receiver<String>| -> Result<usize> {
                            data = rx.recv().context("Writer recv err!")?;
                            Ok(stream.write(data.as_bytes()).context("TcpSteam write err!")?)
                        }).call_box((&data_rx, ))
                );
            };
        });
        Self {
            sender: data_tx,
            handle,
            result: result_rx,
        }
    }

    pub fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}
