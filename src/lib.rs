//! This crate offers modules and functionalities for the rapid implementation of a digital communication node,
//! able to dispatch and forward events and messages between multiple tasks and even between multiple processes or devices,
//! using the offered implementations of some communication protocols.

pub mod framing;
pub mod protocols;

#[cfg(test)]
mod test;

use std::sync::Arc;
use bytes::Bytes;
use tokio::{sync::mpsc::{self, error::{TrySendError, SendError}}, select};
use tokio_util::sync::CancellationToken;
use regex::Regex;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// This struct represents the core dispatching mechanism of the system, and works using a pattern similar to
/// publisher/subscriber.
pub struct Dispatcher {
    subs: Vec<Subscription>,
    rx: mpsc::Receiver<Command>,
    token: CancellationToken,
}

impl Dispatcher {
    /// Creates and runs a new `Dispatcher` instance.
    /// 
    /// # Parameters
    /// - `buffer` : indicates the size of the buffer of the incoming channel of the `Dispatcher`.
    /// 
    /// # Returns
    /// - A tokio::sync::mpsc::Sender<`Command`> to send commands to the `Dispatcher` instance.
    /// - A tokio_util::sync::CancellationToken to handle termination.
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

    // Command receiver
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

    // Dispatch Arc<Event> references to subscribers, while removing dead ones.
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

/// Types of commands valid fo the `Dispatcher`.
#[derive(Clone)]
pub enum Command {
    /// Used for subscribing to the `Dispatcher`.
    Subscribe(Subscription),
    /// Used for forwarding an `Event`.
    Forward(Event),
}

/// Models the subscription of a task to the `Dispatcher`.
#[derive(Clone)]
pub struct Subscription {
    interest: Interest,
    tx: mpsc::Sender<Arc<Event>>,
}

impl Subscription {
    /// Creates a new `Subscription` instance.
    /// 
    /// # Parameters
    /// - `interest` : represents the validation criteria according to which the `Dispatcher` forwards an `Event` to this `Subscription`.
    /// - `buffer` : indicates the size of the buffer of the incoming channel of the `Subscription`.
    /// 
    /// # Returns
    /// - The `Subscription` instance.
    /// - The receiver end of the channel used by the `Dispatcher` to forward the `Event`s.
    pub fn new(interest: Interest, buffer: usize) -> (Self, mpsc::Receiver<Arc<Event>>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self {
            interest,
            tx,
        }, rx)
    }

    /// Creates a `Subscription` with the `new()` method and automatically subscribes it to the given `Dispatcher`.
    /// 
    /// # Parameters
    /// - `interest` : represents the validation criteria according to which the `Dispatcher` forwards an `Event` to this `Subscription`.
    /// - `buffer` : indicates the size of the buffer of the incoming channel of the `Subscription`.
    /// - `dispatcher` : represents the sender linked to the desired `Dispatcher`.
    /// 
    /// # Returns
    /// - The receiver end of the channel used by the `Dispatcher` to forward the `Event`s, wrapped in a `Result`.
    pub async fn subscribe(interest: Interest, buffer: usize, dispatcher: mpsc::Sender<Command>) -> Result<mpsc::Receiver<Arc<Event>>, SendError<Command>> {
        let (sub, rx) = Self::new(interest, buffer);
        dispatcher.send(Command::Subscribe(sub)).await?;
        Ok(rx)
    }

    /// Returns `true` if the `Subscription` channel is not closed, `false` otherwise.
    pub fn is_active(&self) -> bool {
        !self.tx.is_closed()
    }

    /// Forwards the input `Arc<Event>` through the `Subscription` channel if the `Event` is valid for the `Subscription`.
    pub fn forward(&self, event: Arc<Event>) -> Option<Result<(), TrySendError<Arc<Event>>>> {
        if self.is_valid(event.as_ref()) {
            return Some(self.tx.try_send(event));
        }
        None
    }

    /// Returns `true` is the `Event` is valid according to the `Interest` of the `Subscription`, `false` otherwise.
    pub fn is_valid(&self, event: &Event) -> bool {
        self.interest.is_valid(event)
    }
}

/// Represents the interest of a `Subscription` in a certain class of `Event`s.
#[derive(Clone)]
pub struct Interest {
    validator: Regex,
}

impl Interest {
    /// Creates a new `Interest` instance using a `Regex` as validator.
    pub fn new(validator: Regex) -> Self {
        Self {
            validator,
        }
    }

    /// Returns `true` if the `event.topic` matches the regex pattern.
    pub fn is_valid(&self, event: &Event) -> bool {
        self.validator.is_match(&event.topic)
    }
}

/// This struct represents the generic messages of the system.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Event {
    /// Describe the topic of the `Event`.
    pub topic: String,
    /// Represents the instant in which the `Event` was created.
    pub timestamp: DateTime<Utc>,
    /// Contains the raw data of the `Event`.
    pub data: Bytes,
}

impl Event {
    /// Creates a new `Event` instance with a `timestamp` equal to the current instant.
    pub fn new(topic: &str, data: Bytes) -> Self {
        Self {
            topic: String::from(topic),
            timestamp: Utc::now(),
            data,
        }
    }
}