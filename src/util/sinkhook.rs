use futures::{Poll, Sink, StartSend};

#[derive(Clone, Debug)]
#[must_use = "sinks do nothing unless polled"]
pub struct SinkHook<S, F> {
    sink: S,
    hook: F,
    started: Option<()>,
}

impl<S: Sink, F: Fn()> SinkHook<S, F> {
    pub fn new(sink: S, hook: F) -> Self {
        Self {
            sink,
            hook,
            started: None,
        }
    }
}

impl<S: Sink, F: Fn()> Sink for SinkHook<S, F> {
    type SinkItem = S::SinkItem;
    type SinkError = S::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.sink.start_send(item).map(|x| {
            if x.is_ready() {
                self.started = Some(());
            }
            x
        })
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.poll_complete().map(|x| {
            if x.is_ready() && self.started.take().is_some() {
                (self.hook)();
            }
            x
        })
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.sink.close()
    }
}
