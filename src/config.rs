use std::{fs, path::Path, error::Error};

use regex::Regex;
use tokio::{select, sync::mpsc};
use tokio_util::sync::CancellationToken;
use toml;
use serde::{Serialize, Deserialize};

use crate::{protocols::{Protocol, tcp, udp}, Interest, Subscription, Command};

#[derive(Serialize, Deserialize)]
pub struct Config {
 // pub graph: Option<Graph>,
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

pub fn get_configs(path: &str) -> Result<Vec<Config>, Box<dyn Error>> {
    let mut configs: Vec<Config> = Vec::new();
    if Path::new(path).is_dir() {
        let paths = fs::read_dir(path)?;
        for file_path in paths {
            if let Ok(config) = parse_config(file_path?.path()) {
                configs.push(config);
            }
        }
    } else {
        if let Ok(config) = parse_config(path) {
            configs.push(config);
        }
    }
    Ok(configs)
}

fn parse_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
    let toml_str: String = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&toml_str)?;
    Ok(config)
}

//TODO: implement logs
pub async fn init_connections(path: &str, dispatcher: mpsc::Sender<Command>, buffer: usize, token: CancellationToken) -> Result<(), Box<dyn Error>> {
    let configs = get_configs(path)?;
    for config in configs {
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

fn launch_sender(protocol: Protocol, address: &String, interest: Interest, buffer: usize, disp_tx: mpsc::Sender<Command>, token: CancellationToken) {
    let addr = address.clone();
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