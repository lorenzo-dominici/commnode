use std::fs;

use tokio::{self, sync::mpsc::Sender};

use crate::{
    *,
    protocols::*,
    config::*,
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
    let token = CancellationToken::new();
    let tx = Dispatcher::new(32, token.clone());
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

#[test]
fn config() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            config_run().await;
        });
}

async fn config_run() {
    let config = Config {
        receiver: Some(Receiver {
            adv_topic: "".to_string(),
            adv_interest: "".to_string(),
            node: Node {
                channels: vec![
                    Channel {
                        address: "127.0.0.1:8000".to_string(),
                        protocol: Protocol::TCP,
                        interest: r"^TCP$".to_string(),
                    },
                    Channel {
                        address: "127.0.0.1:8001".to_string(),
                        protocol: Protocol::UDP,
                        interest: r"^UDP$".to_string(),
                    }
                ]
            },
        }),
        sender: Some(Node {
                channels: vec![
                    Channel {
                        address: "127.0.0.1:8010".to_string(),
                        protocol: Protocol::TCP,
                        interest: r"^TCP 1$".to_string(),
                    },
                    Channel {
                        address: "127.0.0.1:8011".to_string(),
                        protocol: Protocol::UDP,
                        interest: r"^UDP 1$".to_string(),
                    },
                    Channel {
                        address: "127.0.0.1:8020".to_string(),
                        protocol: Protocol::TCP,
                        interest: r"^TCP 2$".to_string(),
                    },
                    Channel {
                        address: "127.0.0.1:8021".to_string(),
                        protocol: Protocol::UDP,
                        interest: r"^UDP 2$".to_string(),
                    }
                ]
            }),
    };

    let cfg_str = toml::to_string(&config).unwrap();
    let path = "./test-config.toml";
    std::fs::write(path, &cfg_str).unwrap();

    let token = CancellationToken::new();
    let dispatcher = Dispatcher::new(32, token.clone());

    let mut r_sub_tcp_rx = Subscription::subscribe(Interest::new(Regex::new(r"^TCP$").unwrap()), 32, dispatcher.clone()).await.unwrap();
    let mut r_sub_udp_rx = Subscription::subscribe(Interest::new(Regex::new(r"^UDP$").unwrap()), 32, dispatcher.clone()).await.unwrap();

    let (s1_tcp_tx, mut s1_tcp_rx) = mpsc::channel(32);
    let (s1_udp_tx, mut s1_udp_rx) = mpsc::channel(32);

    let (s2_tcp_tx, mut s2_tcp_rx) = mpsc::channel(32);
    let (s2_udp_tx, mut s2_udp_rx) = mpsc::channel(32);
    
    tcp::new_receiver("127.0.0.1:8010", s1_tcp_tx, token.clone()).await.unwrap();
    udp::new_receiver("127.0.0.1:8011", s1_udp_tx, token.clone()).await.unwrap();

    tcp::new_receiver("127.0.0.1:8020", s2_tcp_tx, token.clone()).await.unwrap();
    udp::new_receiver("127.0.0.1:8021", s2_udp_tx, token.clone()).await.unwrap();

    init_connections(path, false, dispatcher.clone(), 32, token.clone()).await.unwrap();

    let (r_send_tcp_tx, r_send_tcp_rx) = mpsc::channel(32);
    let (r_send_udp_tx, r_send_udp_rx) = mpsc::channel(32);

    tcp::new_sender("127.0.0.1:8000", r_send_tcp_rx).await.unwrap();
    udp::new_sender("127.0.0.1:8001", r_send_udp_rx).await.unwrap();

    let r_events = vec![
        Event::new("TCP", Bytes::from_static("one".as_bytes())),
        Event::new("UDP", Bytes::from_static("two".as_bytes())),
    ];

    let s_events = vec![
        Event::new("TCP 1", Bytes::from_static("three".as_bytes())),
        Event::new("UDP 1", Bytes::from_static("four".as_bytes())),
        Event::new("TCP 2", Bytes::from_static("five".as_bytes())),
        Event::new("UDP 2", Bytes::from_static("six".as_bytes())),
    ];

    for event in r_events {
        r_send_tcp_tx.send(event.clone()).await.unwrap();
        r_send_udp_tx.send(event.clone()).await.unwrap();
    }

    for event in s_events {
        dispatcher.send(Command::Forward(event.clone())).await.unwrap();
    }


    let dispatch = r_sub_tcp_rx.recv().await;
    match dispatch {
        Some(arc) => {
            assert!(arc.as_ref().data.to_vec().ends_with("one".as_bytes()), "{:?}", arc.as_ref().data);
        },
        None => panic!("event one")
    }
    assert!(r_sub_tcp_rx.try_recv().is_err());


    let dispatch = r_sub_udp_rx.recv().await;
    match dispatch {
        Some(arc) => {
            assert!(arc.as_ref().data.to_vec().ends_with("two".as_bytes()), "{:?}", arc.as_ref().data);
        },
        None => panic!("event two")
    }
    assert!(r_sub_udp_rx.try_recv().is_err());

    
    let option = s1_tcp_rx.recv().await;
    match option {
        Some(event) => {
            assert!(event.data.to_vec().ends_with("three".as_bytes()), "{:?}", event.data);
        },
        None => panic!("event three")
    }
    assert!(s1_tcp_rx.try_recv().is_err());

    
    let option = s1_udp_rx.recv().await;
    match option {
        Some(event) => {
            assert!(event.data.to_vec().ends_with("four".as_bytes()), "{:?}", event.data);
        },
        None => panic!("event four")
    }
    assert!(s1_udp_rx.try_recv().is_err());


    let option = s2_tcp_rx.recv().await;
    match option {
        Some(event) => {
            assert!(event.data.to_vec().ends_with("five".as_bytes()), "{:?}", event.data);
        },
        None => panic!("event five")
    }
    assert!(s2_tcp_rx.try_recv().is_err());


    let option = s2_udp_rx.recv().await;
    match option {
        Some(event) => {
            assert!(event.data.to_vec().ends_with("six".as_bytes()), "{:?}", event.data);
        },
        None => panic!("event six")
    }
    assert!(s2_udp_rx.try_recv().is_err());
    

    token.cancel();

    fs::remove_file(path).unwrap();
}