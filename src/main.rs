#[macro_use]
extern crate lazy_static;

mod client;
mod read_types;
mod util;
mod logger;

use client::spawn_client_handler;
use io::BufRead;
use log::info;
use rustbreak::{deser::Ron, FileDatabase};
use serde::{Deserialize, Serialize};
use std::{env, io, net::TcpListener, thread, time::Duration};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Forward {
    hostname: String,
    target: String,
}

lazy_static! {
    static ref FORWARDS_DB: FileDatabase<Vec<Forward>, Ron> = {
        let db = FileDatabase::load_from_path_or_default("forwards.ron")
            .expect("Create database from path");

        db.load().expect("Config to load");

        db
    };
}

fn main() {
    logger::setup().unwrap();

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
                let forwards = FORWARDS_DB.borrow_data().unwrap();

                println!("forwards:");
                for forward in forwards.iter() {
                    println!("{} -> {}", forward.hostname, forward.target);
                }
            }

            "forward" => {
                let hostname = parts.next();
                let target = parts.next();

                if hostname.is_none() || target.is_none() {
                    println!("usage: forward <hostname> <target>");
                } else {
                    FORWARDS_DB
                        .write(|db| {
                            db.push(Forward {
                                hostname: hostname.unwrap().to_string(),
                                target: target.unwrap().to_string(),
                            });
                        })
                        .unwrap();

                    FORWARDS_DB.save().unwrap();
                }
            }

            "reload" => {
                FORWARDS_DB.load().unwrap();
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
