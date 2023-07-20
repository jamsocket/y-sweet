use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc::Sender, task::JoinHandle, time::Instant};

#[derive(Default)]
struct ThrottleInner {
    last: Option<Instant>,
    handle: Option<JoinHandle<()>>,
}

pub struct Throttle {
    freq: Duration,
    sender: Sender<()>,
    inner: Arc<std::sync::Mutex<ThrottleInner>>,
}

impl Throttle {
    pub fn new(freq: Duration, sender: Sender<()>) -> Self {
        Self {
            freq,
            sender,
            inner: Arc::default(),
        }
    }

    pub fn call(&self) {
        tracing::info!("Throttle called");
        let mut inner = self.inner.lock().unwrap();
        if inner.handle.is_some() {
            tracing::info!("Throttle already deferred.");
            return;
        }
        let now = Instant::now();
        if let Some(last) = inner.last {
            if now - last < self.freq {
                tracing::info!("Deferring throttle");
                let freq = self.freq;
                let sender = self.sender.clone();
                let inner_clone = self.inner.clone();
                inner.handle.replace(tokio::spawn(async move {
                    tokio::time::sleep_until(last + freq).await;
                    tracing::info!("Deferred throttle ready.");
                    sender.try_send(()).unwrap();

                    let mut inner_clone = inner_clone.lock().unwrap();
                    inner_clone.last.replace(now);
                    inner_clone.handle.take();
                }));
                return;
            }
        }

        tracing::info!("Persisting.");
        self.sender.try_send(()).unwrap();
        inner.last.replace(now);
    }
}
