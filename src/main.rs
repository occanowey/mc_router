#[macro_use]
extern crate lazy_static;

mod cli;
mod client;
mod config;
mod logger;

use std::{env, net::TcpListener, sync::RwLock, thread, time::Duration};

use client::spawn_client_handler;
use config::Config;
use tracing::info;

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(Default::default());
}

fn main() {
    let _guard = logger::setup();

    match config::load() {
        Ok(config) => *CONFIG.write().unwrap() = config,
        Err(error) => {
            info!(
                "Couldn't start router, Failed to read config:\n    {}",
                error
            );
            return;
        }
    }

    let address = env::args().nth(1).expect("address required");

    thread::Builder::new()
        .name("server".to_string())
        .spawn(move || start_server(&address))
        .unwrap();

    cli::start();
}

fn start_server(address: &str) {
    info!("Starting router rev:{}...", git_version::git_version!());
    thread::sleep(Duration::from_millis(250));

    let listener = TcpListener::bind(address).unwrap();
    info!("Listening on {}", address);

    for stream in listener.incoming() {
        spawn_client_handler(stream.unwrap());
    }
}
