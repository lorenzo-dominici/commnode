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
    println!("");

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

enum ReqOutcome {
    Disconnected,
    Crashed,
    Processed,
}

async fn handle_connection(stream: FramedString<TcpStream>, dispatcher: Sender<Command>, token: CancellationToken) {
    let (tx, mut rx) = stream.split();
    let tx = Arc::new(Mutex::new(tx));
    loop {
        select! {
            _ = token.cancelled() => break,
            option = rx.next() => {
                let mut req_outcome = ReqOutcome::Disconnected;
                if let Some(result) = option {
                    req_outcome = ReqOutcome::Crashed;
                    if let Ok(bytes) = result {
                        if let Ok(str_ref) = std::str::from_utf8(&bytes) {
                            if let Ok(request) = toml::from_str::<Request>(str_ref) {
                                log(Color::Text, "handling request... ");
                                let response = {
                                    let result = handle_request(tx.clone(), request, dispatcher.clone(), token.clone()).await;
                                    if result.is_err() {
                                        break;
                                    }
                                    result.unwrap()
                                };
                                logln(Color::Ok, "ok");
                                log(Color::Text, "sending response... ");
                                if !response.ress.is_empty() {
                                    if let Ok(string) = toml::to_string(&response) {
                                        if tx.lock().await.send(Bytes::from(string)).await.is_ok() {
                                            req_outcome = ReqOutcome::Processed;
                                            logln(Color::Ok, "ok")
                                        }
                                    }
                                } else {
                                    req_outcome = ReqOutcome::Processed;
                                    logln(Color::Warn, "unnecessary");
                                }
                            }
                        }
                    }
                }
                match req_outcome {
                    ReqOutcome::Processed => {},
                    ReqOutcome::Crashed => {
                        logln(Color::Err, "bridge session ended! [crashed]");
                        break;
                    },
                    ReqOutcome::Disconnected => {
                        logln(Color::Warn, "bridge session ended! [disconnected]");
                        break;
                    }
                };
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
            if let Some(recvs) = request.recvs {
                for recv in recvs {
                    tokio::spawn(launch_recv(tx.clone(), recv, dispatcher.clone(), clone.clone()));
                }
            }
            let mut ress = Vec::new();
            if let Some(sends) = request.sends {
                for send in &sends {
                    if let Some(expect) = &send.expect {
                        let rx = Subscription::subscribe(Interest::new(Regex::new(&expect.recv.interest).unwrap()), expect.recv.num.into(), dispatcher.clone()).await.unwrap();
                        handles.push(tokio::spawn(launch_n_recvs(expect.recv.clone(), rx)));
                    }
                }
                for send in sends {
                    let packet = Packet {
                        topic: match send.expect {
                            Some(exp) => Some(exp.topic),
                            None => None,
                        },
                        data: send.data
                    };
                    dispatcher.send(Command::Forward(Event::new(&send.topic, Bytes::from(toml::to_string(&packet)?)))).await?;
                }
                for handle in handles {
                    let res = handle.await?;
                    if let Some(r) = res {
                        ress.push(r);
                    }
                }
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
                            let mut rx = Subscription::subscribe(Interest::new(Regex::new(&recv.interest).unwrap()), 32, dispatcher.clone()).await.unwrap();
                            while let Some(arc) = rx.recv().await {
                                let res = Res {
                                    id: recv.id.clone(),
                                    packets: vec![toml::from_str(std::str::from_utf8(&arc.data).unwrap()).unwrap()],
                                };
                                let response = Response {
                                    ress: vec![res],
                                };
                                let bytes: Bytes = toml::to_string(&response).unwrap().as_bytes().to_vec().into();
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
                                        packets: vec![toml::from_str(std::str::from_utf8(&arc.data).unwrap()).unwrap()],
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

async fn launch_n_recvs(recv: Recv, mut rx: tokio::sync::mpsc::Receiver<Arc<Event>>) -> Option<Res> {
    let mut packets = Vec::with_capacity(recv.num.into());
    if recv.num == 0 {
        return None
    }
    for _ in 0..recv.num {
        let event = rx.recv().await.ok_or(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "comms dropped")).unwrap();
        packets.push(toml::from_str(std::str::from_utf8(&event.data).unwrap()).unwrap());
    }
    Some(Res {
        id: recv.id,
        packets,
    })
}

#[derive(Debug, Serialize, Deserialize)]
struct BridgeConfig {
    pub dispatcher_buffer: usize,
    pub channels_size: usize,
    
    pub configs_path: String,
    pub sockets: Vec<String>,   
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    pub sends: Option<Vec<Send>>,
    pub recvs: Option<Vec<Recv>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Send {
    pub topic: String,
    pub data: Vec<u8>,
    pub expect: Option<Expect>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Recv {
    pub id: String,
    pub interest: String,
    pub num: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Expect {
    pub topic: String,
    pub recv: Recv,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Packet {
    pub topic: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    pub ress: Vec<Res>
}

#[derive(Debug, Serialize, Deserialize)]
struct Res {
    pub id: String,
    pub packets: Vec<Packet>,
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
            Self::Text => "\x1b[0m",
            Self::Ok => "\x1b[92m",
            Self::Warn => "\x1b[93m",
            Self::Err => "\x1b[91m",
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