use commnode::{*, config::*, framing::*};
use futures::{StreamExt, SinkExt, stream::SplitSink};
use regex::Regex;
use tokio::{self, sync::{mpsc::Sender, Mutex}, select, signal, net::{TcpListener, TcpStream}};
use tokio_util::sync::CancellationToken;
use std::{env, fmt, sync::Arc};
use serde::{Serialize, Deserialize};
use toml;
use bytes::Bytes;

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
    let result = commnode::config::init_connections(&config.configs_path, true, dispatcher.clone(), config.channels_size, token.clone()).await;
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
    for socket in config.sockets {
        tokio::spawn(init_bridge(socket, dispatcher.clone(), token.clone()));
    }
}

async fn init_bridge(socket: String, dispatcher: Sender<Command>, token: CancellationToken) {
    if let Ok(listener) = TcpListener::bind(socket).await {
        loop {
            select! {
                _ = token.cancelled() => break,
                result = listener.accept() => {
                    if let Ok((stream, _)) = result {
                        let stream = frame_string(stream);
                        tokio::spawn(handle_connection(stream, dispatcher.clone(), token.child_token()));
                    }
                }
            }
        }
    }
}

async fn handle_connection(stream: FramedString<TcpStream>, dispatcher: Sender<Command>, token: CancellationToken) {
    let (tx, mut rx) = stream.split();
    let tx = Arc::new(Mutex::new(tx));
    loop {
        select! {
            _ = token.cancelled() => break,
            option = rx.next() => {
                let mut go_on = false;
                if let Some(result) = option {
                    if let Ok(bytes) = result {
                        if let Ok(str_ref) = std::str::from_utf8(&bytes) {
                            if let Ok(request) = toml::from_str::<Request>(str_ref) {
                                let response = {
                                    let result = handle_request(tx.clone(), request, dispatcher.clone(), token.clone()).await;
                                    if result.is_err() {
                                        break;
                                    }
                                    result.unwrap()
                                };
                                if let Ok(string) = toml::to_string(&response) {
                                    if tx.lock().await.send(string.into()).await.is_ok() {
                                        go_on = true;
                                    }
                                }
                            }
                        }
                    }
                }
                if !go_on {
                    logln(Color::Warn, "request session crahsed!");
                    break;
                }
            },
        }
    }
}

async fn handle_request(tx: Arc<Mutex<SplitSink<FramedString<TcpStream>, Bytes>>>, request: Request, dispatcher: Sender<Command>, token: CancellationToken) -> Result<Response, Box<dyn std::error::Error>> {
    let clone = token.clone();
    select! {
        _ = token.cancelled() => {Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "token cancelled"))?},
        result = async move {
                let mut handles = Vec::new();
                for recv in request.recvs {
                    tokio::spawn(launch_recv(tx.clone(), recv, dispatcher.clone(), clone.clone()));
                }
                for send in request.sends {
                    if let Some(recv) = send.expect {
                        handles.push(tokio::spawn(launch_n_recvs(recv, dispatcher.clone())));
                    }
                }
                let mut ress = Vec::with_capacity(handles.len());
                for handle in handles {
                    ress.push(handle.await?);
                }
                Ok(Response { ress })
            } => {result}
    }
}

async fn launch_recv(tx: Arc<Mutex<SplitSink<FramedString<TcpStream>, Bytes>>>, recv: Recv, dispatcher: Sender<Command>, token: CancellationToken) {
    select! {
        _ = token.cancelled() => {()},
        _ = async move {
                    if recv.num == 0 {
                        tokio::spawn(async move {
                            let mut rx = Subscription::subscribe(Interest::new(Regex::new(&recv.interest).unwrap()), recv.num.into(), dispatcher.clone()).await.unwrap();
                            while let Some(arc) = rx.recv().await {
                                let res = Res {
                                    id: recv.id.clone(),
                                    events: vec![arc.as_ref().clone()],
                                };
                                let response = Response {
                                    ress: vec![res],
                                };
                                let bytes: Bytes = toml::to_string(&response).unwrap().into();
                                tx.lock().await.send(bytes).await.unwrap();
                            }
                        });
                    } else {
                        tokio::spawn(async move {
                            let mut rx = Subscription::subscribe(Interest::new(Regex::new(&recv.interest).unwrap()), recv.num.into(), dispatcher.clone()).await.unwrap();
                            for _ in 0..recv.num {
                                if let Some(arc) = rx.recv().await {
                                    let res = Res {
                                        id: recv.id.clone(),
                                        events: vec![arc.as_ref().clone()],
                                    };
                                    let response = Response {
                                        ress: vec![res],
                                    };
                                    let bytes: Bytes = toml::to_string(&response).unwrap().into();
                                    tx.lock().await.send(bytes).await.unwrap();
                                } else {
                                    break;
                                }
                            }
                        });
                    }
                } => {()}
    }
}

async fn launch_n_recvs(recv: Recv, dispatcher: Sender<Command>) -> Res {
    let mut events = Vec::with_capacity(recv.num.into());
    if recv.num > 0 {
        let mut rx = Subscription::subscribe(Interest::new(Regex::new(&recv.interest).unwrap()), recv.num.into(), dispatcher.clone()).await.unwrap();
        for _ in 0..recv.num {
            let event = rx.recv().await.ok_or(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "comms dropped")).unwrap();
            events.push(event.as_ref().clone());
        }
    }
    Res {
        id: recv.id,
        events,
    }
}

#[derive(Serialize, Deserialize)]
struct BridgeConfig {
    pub dispatcher_buffer: usize,
    pub channels_size: usize,
    
    pub configs_path: String,
    pub sockets: Vec<String>,   
}

#[derive(Serialize, Deserialize)]
struct Request {
    pub sends: Vec<Send>,
    pub recvs: Vec<Recv>,
}

#[derive(Serialize, Deserialize)]
struct Send {
    pub topic: String,
    pub data: String,
    pub expect: Option<Recv>,
}

#[derive(Serialize, Deserialize)]
struct Recv {
    pub id: String,
    pub interest: String,
    pub num: u16,
}

#[derive(Serialize, Deserialize)]
struct Response {
    pub ress: Vec<Res>
}

#[derive(Serialize, Deserialize)]
struct Res {
    pub id: String,
    pub events: Vec<Event>,
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