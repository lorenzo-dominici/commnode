use std::sync::Arc;

use tokio::sync::mpsc::{self, error::TrySendError};

pub struct Dispatcher {
    subs: Vec<Subscription>,
    rx: mpsc::Receiver<Event>,
}

impl Dispatcher {
    pub fn new(buffer: usize) -> mpsc::Sender<Event> {
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
        while let Some(event) = self.rx.recv().await {
            match event {
                Event::Subscribe(sub) => self.subs.push(sub),
                Event::Clear => self.clear(),
                _ => self.dispatch(event),
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
pub enum Event {
    Subscribe(Subscription),
    Clear,
    Raw(Vec<u8>),
}

#[derive(Clone)]
pub struct Subscription {
    topic: Topic,
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
        self.topic.is_valid(event)
    }
}

#[derive(Clone)]
pub struct Topic {
    //TODO: add validation scheme (e.g. regex)
}

impl Topic {
    pub fn is_valid(&self, event: &Event) -> bool {
        //TODO: implement validation
        true
    }
}