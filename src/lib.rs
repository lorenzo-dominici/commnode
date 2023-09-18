mod framing;

use std::sync::Arc;

use tokio::{sync::mpsc::{self, error::TrySendError}, io::{AsyncRead, AsyncWrite}};

use regex::Regex;

use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

pub struct Dispatcher {
    subs: Vec<Subscription>,
    rx: mpsc::Receiver<Command>,
}

impl Dispatcher {
    pub fn new(buffer: usize) -> mpsc::Sender<Command> {
        let (tx, rx) = mpsc::channel(buffer);
        let this = Self {
            subs: Vec::default(),
            rx: rx,
        };

        tokio::spawn(async move {
            this.run().await;
        });
        
        tx
    }

    async fn run(mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                Command::Subscribe(sub) => self.subs.push(sub),
                Command::Clear => self.clear(),
                Command::Forward(event) => self.dispatch(event),
            }
        }
    }
    
    fn clear(&mut self) {
        self.subs.retain(|sub| {
            sub.is_active()
        });
    }

    fn dispatch(&self, event: Event) {
        let arc = Arc::new(event);
        self.subs.iter().for_each(|sub| {
            let _ = sub.forward(arc.clone());
        });
    }
}

#[derive(Clone)]
pub enum Command {
    Subscribe(Subscription),
    Clear,
    Forward(Event),
}

#[derive(Clone)]
pub struct Subscription {
    interest: Interest,
    tx: mpsc::Sender<Arc<Event>>,
}

impl Subscription {
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
    pub data: Data,
}

impl Event {
    pub fn new(topic: &str, data: Data) -> Self {
        Self {
            topic: String::from(topic),
            timestamp: Utc::now(),
            data,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Data {
    String(String),
    Bytes(Vec<u8>),
}