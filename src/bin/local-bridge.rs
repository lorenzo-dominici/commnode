use commnode::{*, config::*};
use tokio::{self, sync::mpsc::Sender, select, signal};
use tokio_util::sync::CancellationToken;
use std::{env, fmt};
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() {
    log(Color::Text, "bridge configuration... ");
    let result = read_toml(env::args_os().skip(1).next().unwrap_or("./config.toml".into()));
    let bridge: BridgeConfig = if let Some(config) = log_unwrap(result) { config } else { return; };
    logln(Color::Ok, "ok");


    log(Color::Text, "dispatcher initialization... ");
    let token = CancellationToken::new();
    let dispatcher = commnode::Dispatcher::new(bridge.dispatcher_buffer, token.clone());
    logln(Color::Ok, "ok");

    log(Color::Text, "commnode configuration... ");
    let result = commnode::config::init_connections(&bridge.configs_path, dispatcher.clone(), bridge.channels_size, token.clone()).await;
    if let None = log_unwrap(result) { return; }
    logln(Color::Ok, "ok");

    log(Color::Text, "local bridge initialization... ");
    let result = init_local_bridge(bridge, dispatcher.clone(), token.clone()).await;
    if let None = log_unwrap(result) {
        return;
    }
    logln(Color::Ok, "ok");

    select! {
        _ = token.cancelled() => logln(Color::Err, "program crashed!"),
        _ = signal::ctrl_c() => logln(Color::Ok, "program terminated!"),
    }

}

async fn init_local_bridge(bridge: BridgeConfig, dispatcher: Sender<Command>, token: CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
    //TODO: main logic
    todo!();
}


#[derive(Serialize, Deserialize)]
struct BridgeConfig {
    pub configs_path: String,
    pub dispatcher_buffer: usize,
    pub channels_size: usize,
    //TODO: ...
}

enum Color {
    Text,
    Ok,
    Warn,
    Err,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Text => "\033[0m",
            Self::Ok => "\033[92m",
            Self::Warn => "\033[93m",
            Self::Err => "\033[91m",
        })
    }
}

fn log(color: Color, text: &str) {
    print!("{}{}{}", color, text, Color::Text)
}

fn logln(color: Color, text: &str) {
    println!("{}{}{}", color, text, Color::Text)
}

fn log_unwrap<T, E: std::fmt::Debug + std::fmt::Display>(result: Result<T, E>) -> Option<T> {
    if let Err(e) = result {
        logln(Color::Err, "failed");
        logln(Color::Err, "program crashed!");
        dbg!(e.to_string());
        return None;
    }
    Some(result.unwrap())
}