extern crate futures;
extern crate tokio;
extern crate chrono;

#[macro_use]
extern crate serde_json;
extern crate serde;

extern crate clap;

use std::collections::HashMap;

use clap::{Arg, App, AppSettings, SubCommand};

mod utils;
mod shared;
mod server;
mod client;

use server::MonamiServer;
use client::{MonamiClient, send_message};
use shared::{MonamiMessage, MonamiControlMessage};
use shared::MessageType;


fn main() {
    // argument parser
    let matches = App::new("Monami")
        .author("Minhwan Kim <azurelysium@gmail.com>")
        .about("This is a simple monitoring tool for remote boxes, my friend")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name("server")
                    .about("runs the monami server")
                    .arg(Arg::with_name("host")
                         .long("host")
                         .value_name("BIND_ADDRESS")
                         .help("Sets a bind address")
                         .default_value("127.0.0.1")
                         .takes_value(true))
                    .arg(Arg::with_name("port")
                         .long("port")
                         .value_name("PORT_NUMBER")
                         .help("Sets a port number to use")
                         .default_value("12345")
                         .takes_value(true))
                    .arg(Arg::with_name("expiration")
                         .long("expiratione")
                         .value_name("SECONDS")
                         .help("Sets a expiration seconds")
                         .default_value("30")
                         .takes_value(true))
                    .arg(Arg::with_name("secret")
                         .long("secret")
                         .value_name("PASSPHRASE")
                         .help("Sets a shared secret")
                         .default_value("minamo")
                         .takes_value(true))
        )
        .subcommand(SubCommand::with_name("client")
                    .about("runs the monami client")
                    .arg(Arg::with_name("host")
                         .long("host")
                         .value_name("SERVER_ADDRESS")
                         .help("Sets a bind address")
                         .default_value("127.0.0.1")
                         .takes_value(true))
                    .arg(Arg::with_name("port")
                         .long("port")
                         .value_name("PORT_NUMBER")
                         .help("Sets a port number to use")
                         .default_value("12345")
                         .takes_value(true))
                    .arg(Arg::with_name("secret")
                         .long("secret")
                         .value_name("PASSPHRASE")
                         .help("Sets a shared secret")
                         .default_value("minamo")
                         .takes_value(true))
                    .arg(Arg::with_name("interval")
                         .long("interval")
                         .value_name("SECONDS")
                         .help("Sets the interval of sending a status update message")
                         .default_value("10")
                         .takes_value(true))
                    .arg(Arg::with_name("tag")
                         .long("tag")
                         .value_name("STRING")
                         .help("Sets a tag")
                         .required(true)
                         .takes_value(true))
                    .arg(Arg::with_name("command")
                         .long("command")
                         .value_name("SHELL_COMMAND")
                         .help("Sets a shell command to execute")
                         .required(true)
                         .takes_value(true))
        )
        .subcommand(SubCommand::with_name("control")
                    .about("executes control commands")
                    .arg(Arg::with_name("host")
                         .long("host")
                         .value_name("SERVER_ADDRESS")
                         .help("Sets a bind address")
                         .default_value("127.0.0.1")
                         .takes_value(true))
                    .arg(Arg::with_name("port")
                         .long("port")
                         .value_name("PORT_NUMBER")
                         .help("Sets a port number to use")
                         .default_value("12345")
                         .takes_value(true))
                    .arg(Arg::with_name("secret")
                         .long("secret")
                         .value_name("PASSPHRASE")
                         .help("Sets a shared secret")
                         .default_value("minamo")
                         .takes_value(true))
                    .arg(Arg::with_name("function")
                         .long("function")
                         .value_name("STRING")
                         .help("Sets a function name to execute")
                         .required(true)
                         .takes_value(true))
                    .arg(Arg::with_name("parameters")
                         .long("parameters")
                         .value_name("JSON_STRING")
                         .help("Sets parameters")
                         .default_value("{}")
                         .takes_value(true))
        )
        .get_matches();

    // Subcommand - server
    if let Some(matches) = matches.subcommand_matches("server") {
        let host: &str = matches.value_of("host").unwrap();
        let port: &str = matches.value_of("port").unwrap();
        let expiration: i64 = matches.value_of("expiration").unwrap().parse().unwrap();
        let secret: &str = matches.value_of("secret").unwrap();

        let srv = MonamiServer::new(host.to_owned(), port.to_owned(), secret.to_owned(), expiration);
        srv.run();
    }

    // Subcommand - client
    if let Some(matches) = matches.subcommand_matches("client") {
        let host: &str = matches.value_of("host").unwrap();
        let port: &str = matches.value_of("port").unwrap();
        let secret: &str = matches.value_of("secret").unwrap();
        let interval: u64 = matches.value_of("interval").unwrap().parse().unwrap();
        let tag: &str = matches.value_of("tag").unwrap();
        let command: &str = matches.value_of("command").unwrap();

        let cli = MonamiClient::new(host.to_owned(), port.to_owned(), secret.to_owned(),
                                    interval, command.to_owned(), tag.to_owned());
        cli.run();
    }

    // Subcommand - control
    if let Some(matches) = matches.subcommand_matches("control") {
        let host: &str = matches.value_of("host").unwrap();
        let port: &str = matches.value_of("port").unwrap();
        let secret: &str = matches.value_of("secret").unwrap();
        let function: &str = matches.value_of("function").unwrap();

        let parameters_str = matches.value_of("parameters").unwrap();
        let parameters: HashMap<String, String> = serde_json::from_str(parameters_str).unwrap();

        let message_control = MonamiControlMessage { function: function.to_owned(), parameters };
        let message = MonamiMessage {
            message_type: MessageType::Control,
            message_status: None,
            message_control: Some(message_control),
        };

        let payload = serde_json::to_string(&message).unwrap();
        tokio::run(send_message(&host, &port, &secret, &payload));
    }
}
