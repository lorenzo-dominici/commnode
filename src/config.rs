use std::{fs, error::Error};

use toml;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub graph: Option<Graph>,
    pub nodes: Vec<Node>,
}

#[derive(Serialize, Deserialize)]
pub struct Graph {
    pub name: String,
    pub meta: Option<String>,
    pub info: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub meta: Option<String>,
    pub info: Option<String>,
    pub channels: Vec<Channel>,
}

#[derive(Serialize, Deserialize)]
pub struct Channel {
    pub address: String,
    pub protocol: String,
}

pub fn parse_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let toml_str = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&toml_str)?;
    Ok(config)
}