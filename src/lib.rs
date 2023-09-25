mod framing;
mod protocols;

#[cfg(test)]
mod test;

use std::sync::Arc;
use bytes::Bytes;
use tokio::{sync::mpsc::{self, error::{TrySendError, SendError}}, select};
use tokio_util::sync::CancellationToken;
use regex::Regex;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub struct Dispatcher {
    subs: Vec<Subscription>,
    rx: mpsc::Receiver<Command>,
    token: CancellationToken,
}

impl Dispatcher {
    pub fn new(buffer: usize) -> (mpsc::Sender<Command>, CancellationToken) {
        let (tx, rx) = mpsc::channel(buffer);
        let token = CancellationToken::new();
        let dispatcher = Self {
            subs: Vec::default(),
            rx: rx,
            token: token.clone(),
        };

        tokio::spawn(async move {
            dispatcher.run().await;
        });
        
        (tx, token)
    }

    async fn run(mut self) {
        loop {
            select! {
                _ = self.token.cancelled() => break,
                Some(cmd) = self.rx.recv() => {
                    match cmd {
                            Command::Subscribe(sub) => self.subs.push(sub),
                            Command::Forward(event) => self.dispatch(event),
                        }
                },
            }
        }
    }

    fn dispatch(&mut self, event: Event) {
        let arc = Arc::new(event);
        self.subs.retain(|sub| {
            if sub.is_active() {
                let _ = sub.forward(arc.clone());
                return true;
            } else {
                return false
            }
        });
    }
}

#[derive(Clone)]
pub enum Command {
    Subscribe(Subscription),
    Forward(Event),
}

#[derive(Clone)]
pub struct Subscription {
    interest: Interest,
    tx: mpsc::Sender<Arc<Event>>,
}

impl Subscription {
    pub fn new(interest: Interest, buffer: usize) -> (Self, mpsc::Receiver<Arc<Event>>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self {
            interest,
            tx,
        }, rx)
    }

    pub async fn subscribe(interest: Interest, buffer: usize, dispatcher: mpsc::Sender<Command>) -> Result<mpsc::Receiver<Arc<Event>>, SendError<Command>> {
        let (sub, rx) = Self::new(interest, buffer);
        dispatcher.send(Command::Subscribe(sub)).await?;
        Ok(rx)
    }

    pub fn is_active(&self) -> bool {
        !self.tx.is_closed()
    }

    pub fn forward(&self, event: Arc<Event>) -> Option<Result<(), TrySendError<Arc<Event>>>> {
        if self.is_valid(event.as_ref()) {
            return Some(self.tx.try_send(event));
        }
        None
    }

    pub fn is_valid(&self, event: &Event) -> bool {
        self.interest.is_valid(event)
    }
}

#[derive(Clone)]
pub struct Interest {
    validator: Regex,
}

impl Interest {
    pub fn new(validator: Regex) -> Self {
        Self {
            validator,
        }
    }

    pub fn is_valid(&self, event: &Event) -> bool {
        self.validator.is_match(&event.topic)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    pub topic: String,
    pub timestamp: DateTime<Utc>,
    pub data: Bytes,
}

impl Event {
    pub fn new(topic: &str, data: Bytes) -> Self {
        Self {
            topic: String::from(topic),
            timestamp: Utc::now(),
            data,
        }
    }
}