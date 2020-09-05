#[macro_use]
extern crate lazy_static;

mod client;
mod config;
mod logger;
mod read_types;
mod util;

use client::spawn_client_handler;
use config::{Config, Forward};
use io::BufRead;
use log::info;
use std::{env, io, net::TcpListener, sync::RwLock, thread, time::Duration};

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(Default::default());
}

fn main() {
    logger::setup().unwrap();

    {
        *CONFIG.write().unwrap() = config::load().unwrap()
    }

    let address = env::args().nth(1).expect("address required");

    thread::Builder::new()
        .name("server".to_string())
        .spawn(move || start_server(&address))
        .unwrap();
    start_cli();
}

fn start_cli() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let mut parts = line.split_whitespace();

        let command = parts.next().unwrap().to_lowercase();
        match command.as_str() {
            "list" => {
                let config = CONFIG.read().unwrap();

                println!("forwards:");
                for forward in config.forwards.iter() {
                    println!("{} -> {}", forward.hostname, forward.target);
                }
            }

            "forward" => {
                let hostname = parts.next();
                let target = parts.next();

                if hostname.is_none() || target.is_none() {
                    println!("usage: forward <hostname> <target>");
                } else {
                    {
                        let mut config = CONFIG.write().unwrap();
                        (*config).forwards.push(Forward {
                            hostname: hostname.unwrap().to_string(),
                            target: target.unwrap().to_string(),
                        });
                    }

                    config::save(&CONFIG.read().unwrap()).unwrap();
                }
            }

            "reload" => {
                *CONFIG.write().unwrap() = config::load().unwrap();
                println!("reloaded forwards");
            }

            _ => println!("Unknown command '{}'", command),
        }
    }
}

fn start_server(address: &str) {
    thread::sleep(Duration::from_millis(250));

    info!("Starting server on {}", address);
    let listener = TcpListener::bind(address).unwrap();

    for stream in listener.incoming() {
        spawn_client_handler(stream.unwrap());
    }
}
