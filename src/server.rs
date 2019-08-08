use std::str;
use std::io::Error;
use std::net::SocketAddr;
use std::collections::HashMap;

use tokio::io;
use tokio::prelude::*;
use tokio::net::UdpSocket;
use tokio::prelude::{AsyncWrite, Future};

use serde::{Deserialize, Serialize};
use serde_json::{Result, Value};
use chrono::Utc;

use crate::shared::{MonamiMessage, MessageType};
use crate::shared::{MonamiStatusMessage, MonamiControlMessage};


#[derive(Serialize, Deserialize, Debug)]
pub struct MonamiNode {
    message: MonamiStatusMessage,
    address: String,
    expires_at: i64,
}

pub struct MonamiServer {
    socket: UdpSocket,
    nodes: HashMap<String, MonamiNode>,
    expiration: i64,
}

impl MonamiServer {
    pub fn new(host: &str, port: &str, expiration: i64) -> MonamiServer {

        let address = format!("{}:{}", host, port);
        let address = address.parse::<SocketAddr>().unwrap();
        let socket = UdpSocket::bind(&address).unwrap();

        let nodes = HashMap::new();
        MonamiServer { socket, nodes, expiration }
    }

    pub fn run(self) {

        struct MonamiServerFuture {
            socket: UdpSocket,
            buf: Vec<u8>,
            to_send: Option<(usize, SocketAddr)>,
            nodes: HashMap<String, MonamiNode>,
            expiration: i64,
        }

        impl Future for MonamiServerFuture {
            type Item = ();
            type Error = ();

            fn poll(&mut self) -> Poll<(), ()> {
                loop {
                    // First we check to see if there's a message we need to echo back.
                    // If so then we try to send it back to the original source, waiting
                    // until it's writable and we're able to do so.
                    if let Some((size, peer)) = self.to_send {
                        let payload_str = str::from_utf8(&self.buf[..size]).unwrap();
                        let message: MonamiMessage = serde_json::from_str(payload_str).unwrap();

                        let mut response = "{\"success\": false}".to_owned();
                        match message.message_type {
                            // Status message
                            MessageType::Status => {
                                let message = message.message_status.unwrap();
                                let now = Utc::now().timestamp();
                                let nodes = &mut self.nodes;
                                let client_uuid = (&message.uuid).to_owned();

                                if nodes.contains_key(&client_uuid) {
                                    // update expires_at
                                    let node = &mut nodes.get_mut(&client_uuid).unwrap();
                                    node.expires_at = now + self.expiration;

                                } else {
                                    // add a newly found node
                                    nodes.insert(client_uuid, MonamiNode {
                                        message,
                                        address: peer.ip().to_string(),
                                        expires_at: now + self.expiration,
                                    });
                                }

                                // remove outdated nodes
                                nodes.retain(|_, v| v.expires_at > now); 

                                // print list of active nodes
                                println!("-- Nodes");
                                for (client_uuid, node) in nodes.iter() {
                                    println!("{}: {} ({})", client_uuid, node.address, node.message.hostname);
                                }

                                response = "{\"success\": true}".to_owned();
                            },

                            // Control message
                            MessageType::Control => {
                                let message = message.message_control.unwrap();

                                if message.function == "list-nodes" {
                                    response = serde_json::to_string(&json!({
                                        "success": true,
                                        "result": &self.nodes,
                                    })).unwrap();
                                } else {
                                    response = serde_json::to_string(&json!({
                                        "success": false,
                                        "reason": "function not supported",
                                    })).unwrap();
                                }
                            }, 
                        }

                        match self.socket.poll_send_to(&response.as_bytes(), &peer) {
                            Ok(Async::Ready(_)) => {
                                self.to_send = None;
                            },
                            Ok(Async::NotReady) => return Ok(Async::NotReady),
                            Err(e) => {
                                println!("Error: {}", e);
                                self.to_send = None;
                            }
                        };
                    }

                    // If we're here then `to_send` is `None`, so we take a look for the
                    // next message we're going to echo back.
                    self.to_send = match self.socket.poll_recv_from(&mut self.buf) {
                        Ok(Async::Ready(val)) => Some(val),
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Err(_) => None
                    };
                }
            }
        }
        let server_future = MonamiServerFuture {
            socket: self.socket,
            buf: vec![0; 1024],
            to_send: None,
            nodes: self.nodes,
            expiration: self.expiration,
        };

        tokio::run(futures::lazy(|| {
            tokio::spawn(server_future);
            Ok(())
        }));
    }
}
