//! This module offers functions to use the TCP communication protocol for sending and receiving `Event`s.

use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use futures::{StreamExt, SinkExt};

use chrono::Utc;

use crate::framing::{FramedStream, frame_stream};
use crate::Event;

/// Runs a new task acting as a listener on a given socket.
/// 
/// # Parameters
/// - `addr` : the socket address of the listener.
/// - `tx` : a transmitter to send back the `Event`s received from the TCP streams.
/// 
/// # Returns
/// - cancellation token for handling termination.
pub async fn new_receiver<T: ToSocketAddrs>(addr: T, tx: mpsc::Sender<Event>, token: CancellationToken) -> Result<(), tokio::io::Error> {
    let listener = TcpListener::bind(addr).await?;
    tokio::spawn(async move {
        listen(listener, tx, token).await;
    });
    Ok(())
}

// Listener task
async fn listen(listener: TcpListener, tx: mpsc::Sender<Event>, token: CancellationToken) {
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
async fn process(mut stream: FramedStream<TcpStream>, tx: mpsc::Sender<Event>, token: CancellationToken) {
    loop {
        select! {
            _ = token.cancelled() => break,
        Some(msg) = stream.next() => {
            if let Ok(event) = msg {
                println!("\x1b[96mIN\x1b[0m [{}] {} - \"{}\" = {} Bytes", Utc::now(), &event.timestamp, &event.topic, event.data.len());
                let _ = tx.send(event).await;
            }
        },
        }
    }
}

/// Runs a new task acting as a TCP sender to a given socket.
/// 
/// # Parameters
/// - `addr` : the socket address of the listener.
/// - `rx` : a receiver to use as the source of the `Event`s to forward to the TCP stream.
pub async fn new_sender<T: ToSocketAddrs>(addr: T, rx: mpsc::Receiver<Event>) -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect(addr).await?;
    let stream = frame_stream(stream);
    tokio::spawn(async move {
        send(stream, rx).await;
    });
    Ok(())
}

// Sender task
async fn send(mut stream: FramedStream<TcpStream>, mut rx: mpsc::Receiver<Event>) {
    while let Some(event) = rx.recv().await {
        println!("\x1b[94mOUT\x1b[0m [{}] {} - \"{}\" = {} Bytes", Utc::now(), &event.timestamp, &event.topic, event.data.len());
        let _ = stream.send(event).await;
    }
}