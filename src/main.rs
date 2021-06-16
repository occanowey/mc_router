#[macro_use]
extern crate lazy_static;

mod cli;
mod client;
mod config;
mod logger;
mod mc_types;

use client::spawn_client_handler;
use config::Config;
use log::info;
use std::{env, net::TcpListener, sync::RwLock, thread, time::Duration};

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

    cli::start();
}

fn start_server(address: &str) {
    thread::sleep(Duration::from_millis(250));

    info!(
        "Starting router rev:{} on {}",
        git_version::git_version!(),
        address
    );
    let listener = TcpListener::bind(address).unwrap();

    for stream in listener.incoming() {
        spawn_client_handler(stream.unwrap());
    }
}
