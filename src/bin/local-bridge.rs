use bytes::BytesMut;
use commnode::{*, config::*};
use tokio::{self, sync::mpsc::Sender, select, signal, net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use tokio_util::sync::CancellationToken;
use std::{env, fmt, net::SocketAddr};
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() {
    log(Color::Text, "bridge configuration... ");
    let result = read_toml(env::args_os().skip(1).next().unwrap_or("./config.toml".into()));
    let config: BridgeConfig = if let Some(config) = log_unwrap(result) { config } else { return; };
    logln(Color::Ok, "ok");


    log(Color::Text, "dispatcher initialization... ");
    let token = CancellationToken::new();
    let dispatcher = commnode::Dispatcher::new(config.dispatcher_buffer, token.clone());
    logln(Color::Ok, "ok");

    log(Color::Text, "commnode configuration... ");
    let result = commnode::config::init_connections(&config.configs_path, dispatcher.clone(), config.channels_size, token.clone()).await;
    if let None = log_unwrap(result) { return; }
    logln(Color::Ok, "ok");

    log(Color::Text, "local bridge initialization... ");
    init_bridges(config, dispatcher.clone(), token.clone());
    logln(Color::Ok, "ok");

    select! {
        _ = token.cancelled() => logln(Color::Err, "program crashed!"),
        _ = signal::ctrl_c() => logln(Color::Ok, "program terminated!"),
    }

}

fn init_bridges(config: BridgeConfig, dispatcher: Sender<Command>, token: CancellationToken) {
    for bridge in config.bridges {
        tokio::spawn(init_bridge(bridge, dispatcher.clone(), token.clone()));
    }
}

async fn init_bridge(bridge: Bridge, dispatcher: Sender<Command>, token: CancellationToken) {
    if let Ok(listener) = TcpListener::bind(&bridge.socket).await {
        loop {
            select! {
                _ = token.cancelled() => break,
                result = listener.accept() => {
                    if let Ok((stream, socket)) = result{
                        tokio::spawn(handle_connection(stream, socket, bridge.clone(), dispatcher.clone(), token.child_token()));
                    }
                }
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream, socket: SocketAddr, bridge: Bridge, dispatcher: Sender<Command>, token: CancellationToken) {
    let mut buffer = BytesMut::with_capacity(2);
    loop {
        select! {
            _ = token.cancelled() => break,
            result = stream.read_buf(&mut buffer) => {
                match result {
                    Ok(bytes_read) => {
                        //TODO: ...
                    },
                    Err(_) => break,
                };
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct BridgeConfig {
    pub dispatcher_buffer: usize,
    pub channels_size: usize,
    
    pub configs_path: String,
    pub bridges: Vec<Bridge>,   
}

#[derive(Clone, Serialize, Deserialize)]
struct Bridge {
    pub socket: String,
    pub src_dir: String,
    pub dest_dir: String,
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