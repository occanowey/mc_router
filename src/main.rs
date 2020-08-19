#[macro_use]
extern crate lazy_static;

mod client;

use client::spawn_client_handler;
use fern::colors::{Color, ColoredLevelConfig};
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
    setup_logger().unwrap();

    let address = env::args().nth(1).expect("address required");

    thread::Builder::new()
        .name("server".to_string())
        .spawn(move || start_server(&address))
        .unwrap();
    start_cli();
}

fn setup_logger() -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::Blue)
        .trace(Color::Cyan);

    let file_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d-%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file("output.log")?);

    let stdout_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d-%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout());

    fern::Dispatch::new()
        .chain(file_logger)
        .chain(stdout_logger)
        .apply()?;

    Ok(())
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
