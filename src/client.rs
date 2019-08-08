extern crate uuid;
extern crate hostname;

use std::io::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime};
use std::process::Command;

use tokio::io;
use tokio::prelude::*;
use tokio::net::UdpSocket;
use tokio::timer::Interval;

use chrono::{Utc, Local, DateTime};
use serde_json::json;

use crate::shared::{MonamiMessage, MonamiStatusMessage};
use crate::shared::{MessageType};
use crate::utils::aes_encrypt;


pub struct MonamiClient {
    id: String,
    hostname: String,
    host: String,
    port: String,
    secret: String,
    interval: u64,
    command: String,
    tag: String,
}

pub fn send_message(host: &str, port: &str, payload_str: &str)
                -> Box<Future<Item = (), Error = ()> + Send> {
    let payload = payload_str.to_owned();

    let address = format!("{}:{}", host, port);
    let remote_address = address.parse::<SocketAddr>().unwrap();

    // We use port 0 to let the operating system allocate an available port for us.
    let address = "0.0.0.0:0";
    let local_address = address.parse::<SocketAddr>().unwrap();

    let socket = UdpSocket::bind(&local_address).unwrap();
    const MAX_DATAGRAM_SIZE: usize = 65_507;

    Box::new(
        socket
            .send_dgram(payload, &remote_address)
            .and_then(|(socket, _)| socket.recv_dgram(vec![0u8; MAX_DATAGRAM_SIZE]))
            .map(|(_, data, len, _)| {
                println!(
                    "Received {} bytes:\n{}",
                    len,
                    String::from_utf8_lossy(&data[..len])
                )
            })
            .map_err(|_| ())
    )
}

impl MonamiClient {
    pub fn new(host: String, port: String, secret: String,
               interval: u64, command: String, tag: String) -> MonamiClient {

        let id = uuid::Uuid::new_v4().to_hyphenated().to_string();
        let hostname = hostname::get_hostname().unwrap_or("-".to_owned());
        MonamiClient { id, hostname, host, port, secret, interval, command, tag }
    }

    pub fn run(self) {
        let monami_client_future = Interval::new(Instant::now(), Duration::from_secs(self.interval))
            .for_each(move |_| {

                let dt: DateTime<Local> = Local::now();
                println!("-- {}", dt.to_string());

                // execute the specified command
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(&self.command)
                    .output()
                    .expect("failed to execute process");

                if output.status.success() {
                    let cmd_output = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    let ts = Utc::now().timestamp();

                    // send execution result to monami server
                    let message_status = MonamiStatusMessage {
                        hostname: self.hostname.to_owned(),
                        uuid: self.id.to_owned(),
                        tag: self.tag.to_owned(),
                        command: self.command.to_owned(),
                        output: cmd_output.to_owned(),
                        timestamp: ts,
                    };
                    let message = MonamiMessage {
                        message_type: MessageType::Status,
                        message_status: Some(message_status),
                        message_control: None,
                    };
                    let payload = serde_json::to_string(&message).unwrap();
                    println!("{}", payload);

                    let encrypted = aes_encrypt(&payload, &self.secret).unwrap();
                    tokio::spawn(send_message(&self.host, &self.port, &encrypted));
                }
                Ok(())
            })
            .map_err(|e| panic!("interval errored; err={:?}", e));

        tokio::run(futures::lazy(|| {
            tokio::spawn(monami_client_future);
            Ok(())
        }));
    }
}
