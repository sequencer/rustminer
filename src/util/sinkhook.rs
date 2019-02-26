use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};

#[derive(Clone, Debug)]
#[must_use = "sinks do nothing unless polled"]
pub struct SinkHook<S, F> {
    sink: S,
    f: F,
    started: Option<()>,
}

impl<S: Sink, F: Fn()> SinkHook<S, F> {
    pub fn new(s: S, f: F) -> Self {
        Self {
            sink: s,
            f,
            started: None,
        }
    }
}

impl<S: Sink, F: Fn()> Sink for SinkHook<S, F> {
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sink.start_send(item).map(|x| {
            if let AsyncSink::Ready = x {
                self.started = Some(());
            }
            x
        })
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete().map(|x| {
            if x == Async::Ready(()) && self.started.take().is_some() {
                (self.f)();
            }
            x
        })
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.close()
    }
}

impl<S: Stream, F> Stream for SinkHook<S, F> {
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<S::Item>, S::Error> {
        self.sink.poll()
    }
}
