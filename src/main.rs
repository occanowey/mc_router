#[macro_use]
extern crate lazy_static;

mod cli;
mod client;
mod config;
mod logger;

use std::{
    env,
    net::{SocketAddr, TcpListener},
    sync::RwLock,
    thread,
    time::Duration,
};

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

    let addr = env::args()
        .nth(1)
        .expect("expected address")
        .parse()
        .expect("address was in unknown format");
    thread::Builder::new()
        .name("server".to_string())
        .spawn(move || run_server(addr))
        .unwrap();

    cli::start();
}

fn run_server(addr: SocketAddr) {
    info!("Starting router rev:{}...", git_version::git_version!());
    thread::sleep(Duration::from_millis(250));

    let listener = TcpListener::bind(addr).unwrap();
    info!("Listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept().unwrap();
        spawn_client_handler(stream, addr);
    }
}
