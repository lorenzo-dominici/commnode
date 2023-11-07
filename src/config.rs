use std::{fs, path::Path, error::Error};

use regex::Regex;
use tokio::{select, sync::mpsc};
use tokio_util::sync::CancellationToken;
use toml;
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{protocols::{Protocol, tcp, udp}, Interest, Subscription, Command};

#[derive(Serialize, Deserialize)]
pub struct Config {
 // pub graph: Option<Graph>,
    pub local: Node,
    pub nodes: Vec<Node>,
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

pub fn read_n_toml(path: &str) -> Result<Vec<Config>, Box<dyn Error>> {
    let mut configs: Vec<Config> = Vec::new();
    if Path::new(path).is_dir() {
        let paths = fs::read_dir(path)?;
        for file_path in paths {
            if let Ok(config) = read_toml(file_path?.path()) {
                configs.push(config);
            }
        }
    } else {
        if let Ok(config) = read_toml(path) {
            configs.push(config);
        }
    }
    Ok(configs)
}

pub fn read_toml<P: AsRef<Path>, T: DeserializeOwned>(path: P) -> Result<T, Box<dyn Error>> {
    let toml_str: String = fs::read_to_string(path)?;
    let toml_struct: T = toml::from_str(&toml_str)?;
    Ok(toml_struct)
}

//TODO: implement logs
pub async fn init_connections(path: &str, dispatcher: mpsc::Sender<Command>, buffer: usize, token: CancellationToken) -> Result<(), Box<dyn Error>> {
    let configs = read_n_toml(path)?;
    for config in configs {
        for channel in config.local.channels {
            let regex = match Regex::new(&channel.interest) {
                Ok(re) => re,
                Err(_) => continue,
            };
            let interest = Interest::new(regex);
            launch_receiver(channel.protocol, &channel.address, interest, buffer, dispatcher.clone(), token.clone());
        }
        for node in config.nodes {
            for channel in node.channels {
                let regex = match Regex::new(&channel.interest) {
                    Ok(re) => re,
                    Err(_) => continue,
                };
                let interest = Interest::new(regex);
                launch_sender(channel.protocol, &channel.address, interest, buffer, dispatcher.clone(), token.clone());
            }
        }
    }
    Ok(())
}

fn launch_receiver(protocol: Protocol, address: &str, interest: Interest, buffer: usize, disp_tx: mpsc::Sender<Command>, token: CancellationToken) {
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
        loop {
            select! {
                _ = token.cancelled() => {},
                dispatch = rx.recv() => {
                    match dispatch {
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
    });
}

fn launch_sender(protocol: Protocol, address: &str, interest: Interest, buffer: usize, disp_tx: mpsc::Sender<Command>, token: CancellationToken) {
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
        loop {
            select! {
                _ = token.cancelled() => {},
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