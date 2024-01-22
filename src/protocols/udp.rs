//! This module offers functions to use the UDP communication protocol for sending and receiving `Event`s.

use std::io::{Error, ErrorKind};

use tokio::net::{ToSocketAddrs, lookup_host};
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use udp_stream::{UdpListener, UdpStream};

use futures::{StreamExt, SinkExt};

use chrono::Utc;

use crate::framing::{FramedStream, frame_stream};
use crate::Event;

/// Runs a new task acting as a listener on a given socket.
/// 
/// # Parameters
/// - `addr` : the socket address of the listener.
/// - `tx` : a transmitter to send back the `Event`s received from the UDP communicaitons.
/// 
/// # Returns
/// - cancellation token for handling termination.
pub async fn new_receiver<T: ToSocketAddrs>(addr: T, tx: mpsc::Sender<Event>, token: CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
    let listener = UdpListener::bind(lookup_host(addr).await?.next().ok_or(Error::new(ErrorKind::InvalidData, "address not found"))?).await?;
    tokio::spawn(async move {
        listen(listener, tx, token).await;
    });
    Ok(())
}

// Listener task
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

// Stream handler
async fn process(mut stream: FramedStream<UdpStream>, tx: mpsc::Sender<Event>, token: CancellationToken) {
    loop {
        select! {
            _ = token.cancelled() => break,
        Some(msg) = stream.next() => {
            if let Ok(event) = msg {
                println!("IN [{}] {} - \"{}\" = {} Bytes", Utc::now(), &event.timestamp, &event.topic, event.data.len());
                let _ = tx.send(event).await;
            }
        },
        }
    }
}

/// Runs a new task acting as a UDP sender to a given socket.
/// 
/// # Parameters
/// - `addr` : the socket address of the listener.
/// - `rx` : a receiver to use as the source of the `Event`s to forward to the UDP channel.
pub async fn new_sender<T: ToSocketAddrs>(addr: T, rx: mpsc::Receiver<Event>) -> Result<(), Box<dyn std::error::Error>> {
    let stream = UdpStream::connect(lookup_host(addr).await?.next().ok_or(Error::new(ErrorKind::InvalidData, "address not found"))?).await?;
    let stream = frame_stream(stream);
    tokio::spawn(async move {
        send(stream, rx).await;
    });
    Ok(())
}

//Sender task
async fn send(mut stream: FramedStream<UdpStream>, mut rx: mpsc::Receiver<Event>) {
    while let Some(event) = rx.recv().await {
        println!("OUT [{}] {} - \"{}\" = {} Bytes", Utc::now(), &event.timestamp, &event.topic, event.data.len());
        let _ = stream.send(event).await;
    }
}