use std::{fs, path::Path, error::Error, sync::Arc};

use bytes::Bytes;
use regex::Regex;
use tokio::{select, sync::mpsc};
use tokio_util::sync::CancellationToken;
use toml;
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{protocols::{Protocol, tcp, udp}, Interest, Subscription, Command, Event};

#[derive(Serialize, Deserialize)]
pub struct Config {
 // pub graph: Option<Graph>,
    pub receiver: Receiver,
    pub sender: Node,
}

/* Possible future implementation
#[derive(Serialize, Deserialize)]
pub struct Graph {
    pub name: String,
    pub meta: Option<String>,
    pub info: Option<String>,
}
*/

#[derive(Serialize, Deserialize)]
pub struct Node {
    // pub name: String,
    // pub meta: Option<String>,
    // pub info: Option<String>,
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize)]
pub struct Channel {
    pub address: String,
    pub protocol: Protocol,
    pub interest: String,
}

#[derive(Serialize, Deserialize)]
pub struct Receiver {
    pub adv_topic: String,
    pub adv_interest: String,
    pub node: Node,
}

pub fn read_n_toml<T: DeserializeOwned>(path: &str) -> Result<Vec<T>, Box<dyn Error>> {
    let mut tomls: Vec<T> = Vec::new();
    if Path::new(path).is_dir() {
        let paths = fs::read_dir(path)?;
        for file_path in paths {
            if let Ok(parsed) = read_toml(file_path?.path()) {
                tomls.push(parsed);
            }
        }
    } else {
        if let Ok(parsed) = read_toml(path) {
            tomls.push(parsed);
        }
    }
    Ok(tomls)
}

pub fn read_toml<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Result<T, Box<dyn Error>> {
    let toml_str: String = fs::read_to_string(path)?;
    let toml_struct: T = toml::from_str(&toml_str)?;
    Ok(toml_struct)
}

//TODO: implement logs
pub async fn init_connections(path: &str, adv: bool, dispatcher: mpsc::Sender<Command>, buffer: usize, token: CancellationToken) -> Result<(), Box<dyn Error>> {
    let configs = read_n_toml::<Config>(path)?;
    for config in configs {
        let recv = Arc::new(config.receiver);
        let redirect = if adv {
            if let Ok(re) = Regex::new(&recv.adv_interest) {
                let (tx, rx) = mpsc::channel(buffer);
                launch_redirect(rx, dispatcher.clone(), buffer, token.clone());
                Some((Interest::new(re), tx))
            } else {
                None
            }
        } else {
            None
        };
        for channel in &recv.node.channels {
            let regex = match Regex::new(&channel.interest) {
                Ok(re) => re,
                Err(_) => continue,
            };
            let interest = Interest::new(regex);
            launch_receiver(redirect.clone(), channel.protocol.clone(), &channel.address, interest, buffer, dispatcher.clone(), token.clone());
        }
        for channel in config.sender.channels {
            let regex = match Regex::new(&channel.interest) {
                Ok(re) => re,
                Err(_) => continue,
            };
            let interest = Interest::new(regex);
            launch_sender(if adv {Some(recv.clone())} else {None}, channel.protocol, &channel.address, interest, buffer, dispatcher.clone(), token.clone());
        }
    }
    Ok(())
}

fn launch_redirect(mut rx: mpsc::Receiver<Event>, disp_tx: mpsc::Sender<Command>, buffer: usize, token: CancellationToken) {
    tokio::spawn(async move {
        loop {
            select! {
                _ = token.cancelled() => break,
                option = rx.recv() => {
                    match option {
                        Some(event) => {
                            if let Ok(string) = std::str::from_utf8(&event.data) {
                                if let Ok(recv) = toml::from_str::<Receiver>(string) {
                                    for channel in recv.node.channels {
                                        if let Ok(re) = Regex::new(&channel.interest) {
                                            launch_sender(None, channel.protocol, &channel.address, Interest::new(re), buffer, disp_tx.clone(), token.clone());
                                        }
                                    }
                                }
                            }
                        },
                        None => break,
                    }
                }
            }
        }
    });
}

fn launch_receiver(send: Option<(Interest, mpsc::Sender<Event>)>, protocol: Protocol, address: &str, interest: Interest, buffer: usize, disp_tx: mpsc::Sender<Command>, token: CancellationToken) {
    let addr = address.to_string().clone();
    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::channel(buffer);
        match protocol {
            Protocol::TCP => {
                tcp::new_receiver(addr, tx, token.clone()).await.unwrap();
            },
            Protocol::UDP => {
                udp::new_receiver(addr, tx, token.clone()).await.unwrap();
            },
        };
        if let Some((adv, tx)) = send {
            loop {
                select! {
                    _ = token.cancelled() => break,
                    message = rx.recv() => {
                        match message {
                            Some(event) => {
                                if adv.is_valid(&event) {
                                    let _ = tx.send(event).await;
                                } else if interest.is_valid(&event) {
                                    disp_tx.send(Command::Forward(event)).await.unwrap();
                                }
                            },
                            None => break,
                        }
                    }
                }
            }
        } else {
            loop {
                select! {
                    _ = token.cancelled() => break,
                    message = rx.recv() => {
                        match message {
                            Some(event) => {
                                if interest.is_valid(&event) {
                                    disp_tx.send(Command::Forward(event)).await.unwrap();
                                }
                            },
                            None => break,
                        }
                    }
                }
            }
        }

    });
}

fn launch_sender(recv: Option<Arc<Receiver>>, protocol: Protocol, address: &str, interest: Interest, buffer: usize, disp_tx: mpsc::Sender<Command>, token: CancellationToken) {
    let addr = address.to_string().clone();
    tokio::spawn(async move {
        let (sub, mut arc_rx) = Subscription::new(interest, buffer);
        let (tx, rx) = mpsc::channel(buffer);
        match protocol {
            Protocol::TCP => {
                tcp::new_sender(addr, rx).await.unwrap();
            },
            Protocol::UDP => {
                udp::new_sender(addr, rx).await.unwrap();
            },
        };
        disp_tx.send(Command::Subscribe(sub)).await.unwrap();
        if let Some(receiver) = recv {
            let string = toml::to_string(receiver.as_ref()).unwrap();
            let event = Event::new(&receiver.adv_topic, Bytes::from(string));
            tx.send(event).await.unwrap();
        }
        loop {
            select! {
                _ = token.cancelled() => break,
                dispatch = arc_rx.recv() => {
                    match dispatch {
                        Some(event) => {
                            tx.send(event.as_ref().clone()).await.unwrap();
                        },
                        None => break,
                    }
                }
            }
        }
    });
}