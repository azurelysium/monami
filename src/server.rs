use std::str;
use std::net::{SocketAddr, ToSocketAddrs};
use std::collections::HashMap;

use tokio::prelude::*;
use tokio::net::UdpSocket;
use tokio::prelude::Future;

use serde::{Deserialize, Serialize};
use chrono::{Utc, Local, DateTime};

use crate::shared::{MonamiMessage, MessageType};
use crate::shared::MonamiStatusMessage;
use crate::utils::{aes_encrypt, aes_decrypt};


#[derive(Serialize, Deserialize, Debug)]
pub struct MonamiNode {
    message: MonamiStatusMessage,
    address: String,
    expires_at: i64,
}

pub struct MonamiServer {
    secret: String,
    socket: UdpSocket,
    nodes: HashMap<String, MonamiNode>,
    expiration: i64,
}

impl MonamiServer {
    pub fn new(host: String, port: String, secret: String, expiration: i64) -> MonamiServer {

        let address = format!("{}:{}", host, port);
        let address = address.to_socket_addrs().unwrap().next().unwrap();
        let socket = UdpSocket::bind(&address).unwrap();

        let nodes = HashMap::new();
        MonamiServer { secret, socket, nodes, expiration }
    }

    pub fn run(self) {

        struct MonamiServerFuture {
            server: MonamiServer,
            buf: Vec<u8>,
            to_send: Option<(usize, SocketAddr)>,
        }

        impl Future for MonamiServerFuture {
            type Item = ();
            type Error = ();

            fn poll(&mut self) -> Poll<(), ()> {
                loop {
                    if let Some((size, peer)) = self.to_send {
                        let payload_str = str::from_utf8(&self.buf[..size]).unwrap();

                        let mut message = MonamiMessage {
                            message_type: MessageType::Invalid,
                            message_status: None,
                            message_control: None,
                        };

                        if let Ok(decrypted) = aes_decrypt(payload_str, &self.server.secret) {
                            message = serde_json::from_str(&decrypted).unwrap();
                        }

                        let mut response = "{\"success\": false}".to_owned();
                        match message.message_type {
                            MessageType::Invalid => {},

                            // Status message
                            MessageType::Status => {
                                let message = message.message_status.unwrap();
                                let now = Utc::now().timestamp();
                                let nodes = &mut self.server.nodes;
                                let client_uuid = (&message.uuid).to_owned();

                                // update expiration/output/timestamp or insert a newly found node
                                let expires_at = now + self.server.expiration;
                                nodes.entry(client_uuid)
                                    .and_modify(|e| {
                                        e.expires_at = expires_at;
                                        e.message.timestamp = now;
                                        e.message.output = message.output.to_owned();
                                    })
                                    .or_insert(MonamiNode {
                                        message,
                                        address: peer.ip().to_string(),
                                        expires_at,
                                    });

                                // remove outdated nodes
                                nodes.retain(|_, v| v.expires_at > now); 

                                // print list of active nodes
                                let dt: DateTime<Local> = Local::now();
                                println!("-- {}", dt.to_string());

                                for (client_uuid, node) in nodes.iter() {
                                    println!("{}: {} ({}) [{}]",
                                             client_uuid, node.address, node.message.hostname,
                                             node.message.tag);
                                }

                                response = "{\"success\": true}".to_owned();
                            },

                            // Control message
                            MessageType::Control => {
                                let message = message.message_control.unwrap();
                                let now = Utc::now().timestamp();

                                if message.function == "list-nodes" {
                                    let vec_nodes: Vec<&MonamiNode> = if message.parameters.contains_key("tag") {
                                        let tag = message.parameters.get("tag").unwrap().to_owned();
                                        self.server.nodes.values().filter(|v| v.message.tag == tag).collect()
                                    } else {
                                        self.server.nodes.values().collect()
                                    };

                                    response = serde_json::to_string(&json!({
                                        "success": true,
                                        "result": json!({
                                            "nodes": &vec_nodes,
                                            "timestamp": now,
                                        }),
                                    })).unwrap();
                                } else {
                                    response = serde_json::to_string(&json!({
                                        "success": false,
                                        "reason": "function not supported",
                                    })).unwrap();
                                }
                            }, 
                        }

                        let encrypted = aes_encrypt(&response, &self.server.secret).unwrap();
                        match self.server.socket.poll_send_to(&encrypted.as_bytes(), &peer) {
                            Ok(Async::Ready(_)) => {
                                self.to_send = None;
                            },
                            Ok(Async::NotReady) => return Ok(Async::NotReady),
                            Err(e) => {
                                println!("poll_send_to error: {}", e);
                                self.to_send = None;
                            }
                        };
                    }

                    // If we're here then `to_send` is `None`, so we take a look for the
                    // next message we're going to echo back.
                    self.to_send = match self.server.socket.poll_recv_from(&mut self.buf) {
                        Ok(Async::Ready(val)) => Some(val),
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Err(e) => {
                            println!("poll_recv_from error: {}", e);
                            None
                        }
                    };
                }
            }
        }
        let server_future = MonamiServerFuture {
            server: self,
            buf: vec![0; 40960],
            to_send: None,
        };

        tokio::run(futures::lazy(|| {
            tokio::spawn(server_future);
            Ok(())
        }));
    }
}
