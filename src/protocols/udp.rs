use std::io::{Error, ErrorKind};

use tokio::net::{ToSocketAddrs, lookup_host};
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use udp_stream::{UdpListener, UdpStream};

use futures::{StreamExt, SinkExt};

use crate::framing::{FramedStream, frame_stream};
use crate::Event;

pub async fn new_receiver<T: ToSocketAddrs>(addr: T, tx: mpsc::Sender<Event>) -> Result<CancellationToken, Box<dyn std::error::Error>> {
    let listener = UdpListener::bind(lookup_host(addr).await?.next().ok_or(Error::new(ErrorKind::InvalidData, "address not found"))?).await?;
    let token = CancellationToken::new();
    let clone = token.clone();
    tokio::spawn(async move {
        listen(listener, tx, clone).await;
    });
    Ok(token)
}

async fn listen(listener: UdpListener, tx: mpsc::Sender<Event>, token: CancellationToken) {
    loop {
        select! {
            _ = token.cancelled() => break,
            Ok((stream, _)) = listener.accept() => {
                let stream = frame_stream(stream);
                let clone = tx.clone();
                let child = token.child_token();
                tokio::spawn(async move {
                    process(stream, clone, child).await;
                });
            },
        }
    }
}

async fn process(mut stream: FramedStream<UdpStream>, tx: mpsc::Sender<Event>, token: CancellationToken) {
    loop {
        select! {
            _ = token.cancelled() => break,
        Some(msg) = stream.next() => {
            if let Ok(event) = msg {
                let _ = tx.send(event).await;
            }
        },
        }
    }
}

pub async fn new_sender<T: ToSocketAddrs>(addr: T, rx: mpsc::Receiver<Event>) -> Result<(), Box<dyn std::error::Error>> {
    let stream = UdpStream::connect(lookup_host(addr).await?.next().ok_or(Error::new(ErrorKind::InvalidData, "address not found"))?).await?;
    let stream = frame_stream(stream);
    tokio::spawn(async move {
        send(stream, rx).await;
    });
    Ok(())
}

async fn send(mut stream: FramedStream<UdpStream>, mut rx: mpsc::Receiver<Event>) {

    while let Some(event) = rx.recv().await {
        let _ = stream.send(event).await;
    }
}