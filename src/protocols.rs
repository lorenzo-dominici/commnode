use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::sync::mpsc;

use crate::framing::FramedStream;

use super::framing::frame_stream;
use super::{Command, Event, Interest};
use futures::{StreamExt, SinkExt};

pub struct Tcp {
}

impl Tcp {
    pub async fn send_once<T: ToSocketAddrs>(addr: T, event: Event) -> Result<(), tokio::io::Error> {
        let stream = TcpStream::connect(addr).await?;
        let mut stream = frame_stream(stream);

        match stream.send(event).await {
            Ok(_) => {
                let _ = stream.close();
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    pub async fn new_listener(addr: String, dispatcher: mpsc::Sender<Command>, interest: Interest) -> Result<tokio::task::JoinHandle<()>, tokio::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Ok(tokio::spawn(async move {
            Self::run(listener, dispatcher).await;
        }))
    }

    async fn run(listener: TcpListener, dispatcher: mpsc::Sender<Command>) {
        while let Ok((stream, _)) = listener.accept().await {
            let mut stream = frame_stream(stream);
            let tx = dispatcher.clone();
            tokio::spawn(async move {
                Self::process(stream, tx).await;
            });
        }
    }

    async fn process(mut stream: FramedStream<TcpStream>, dispatcher: mpsc::Sender<Command>) {
        while let Some(msg) = stream.next().await {
            if let Ok(event) = msg {
                let _ = dispatcher.send(Command::Forward(event)).await;
            }
        }
    }
}