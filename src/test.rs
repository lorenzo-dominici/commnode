use tokio::{self, sync::mpsc::Sender};

use crate::{
    *,
    protocols::*,
};

#[test]
fn local() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            local_run().await;
        });
}

async fn local_run() {
    let (tx, _) = Dispatcher::new(32);
    tokio::spawn(local_process(tx.clone()));
}

async fn local_process(tx: Sender<Command>) {
    let interest = Interest::new(Regex::new(r"^test[0-9]$").unwrap());
    let mut rx = Subscription::subscribe(interest, 32, tx.clone()).await.unwrap();
    let event = Event::new("test0", Bytes::from_static("success".as_bytes()));
    tx.send(Command::Forward(event)).await.unwrap();
    let msg = rx.recv().await.unwrap();
    assert!(msg.as_ref().data.to_vec().ends_with("success".as_bytes()));
}

#[test]
fn remote_tcp() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            remote_tcp_run().await;
        });
}

async fn remote_tcp_run() {
    tokio::spawn(remote_tcp_server_process());
    tokio::spawn(remote_tcp_client_process());
}

async fn remote_tcp_server_process() {
    let (tx, mut rx) = mpsc::channel(32);
    let token = CancellationToken::new();
    tcp::new_receiver("127.0.0.1:8080", tx, token.clone()).await.unwrap();
    let event = rx.recv().await.unwrap();
    assert!(event.data.to_vec().ends_with("success".as_bytes()));
    token.cancel()
}

async fn remote_tcp_client_process() {
    let (tx, rx) = mpsc::channel(32);
    tcp::new_sender("127.0.0.1:8080", rx).await.unwrap();
    let event = Event::new("test0", Bytes::from_static("success".as_bytes()));
    tx.send(event).await.unwrap();
}

#[test]
fn remote_udp() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            remote_udp_run().await;
        });
}

async fn remote_udp_run() {
    tokio::spawn(remote_udp_server_process());
    tokio::spawn(remote_udp_client_process());
}

async fn remote_udp_server_process() {
    let (tx, mut rx) = mpsc::channel(32);
    let token = CancellationToken::new();
    udp::new_receiver("127.0.0.1:8080", tx, token.clone()).await.unwrap();
    let event = rx.recv().await.unwrap();
    assert!(event.data.to_vec().ends_with("success".as_bytes()));
    token.cancel()
}

async fn remote_udp_client_process() {
    let (tx, rx) = mpsc::channel(32);
    udp::new_sender("127.0.0.1:8080", rx).await.unwrap();
    let event = Event::new("test0", Bytes::from_static("success".as_bytes()));
    tx.send(event).await.unwrap();
}