#[macro_use]
extern crate lazy_static;

use env_logger::Env;
use io::BufRead;
use log::{debug, info};
use rustbreak::{deser::Ron, FileDatabase};
use serde::{Deserialize, Serialize};
use std::{
    env,
    io::{self, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
};

fn read_varint(offset: usize, src: &[u8]) -> (i32, usize) {
    let mut acc = 0;
    let mut i = 0;

    loop {
        let byte = src[offset + i] as i32;
        acc |= (byte & 0x7F) << (i * 7);

        i += 1;
        if i > 5 {
            panic!("varint too big");
        }

        if (byte & 0b10000000) == 0 {
            break;
        }
    }

    (acc, i)
}

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
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    thread::spawn(start_server);
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

fn start_server() {
    let address = env::var("MCR_ADDRESS").expect("Address required");

    info!("Starting server on {}", address);
    let listener = TcpListener::bind(address).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_client(stream);
    }
}

fn handle_client(client: TcpStream) {
    let client_address = client.local_addr().unwrap();
    info!("New connection from {}", client_address);

    let mut client = BufReader::new(client);

    let mut index = 0;
    // 7 because it's the minimum length of a handshake packet but more than a varint :)
    let mut buffer = vec![0u8; 7];
    client.read_exact(&mut buffer).unwrap();

    // todo: handle legacy ping

    let (length, length_length) = read_varint(index, &buffer);
    index += length_length;

    // length of the packet - what's already in the buffer
    let chunk_length = (length as usize) - (buffer.len() - length_length);
    let mut chunk = vec![0; chunk_length];
    client.read_exact(&mut chunk).unwrap();
    buffer.append(&mut chunk);

    let (id, id_length) = read_varint(index, &buffer);
    index += id_length;

    if id != 0 {
        panic!("Invalid packet id recieved from client.");
    }

    let (protocol_version, protocol_version_length) = read_varint(index, &buffer);
    index += protocol_version_length;

    // messy
    let server_address = {
        let (length, length_length) = read_varint(index, &buffer);
        index += length_length;
        let data = &buffer[index..index + length as usize];
        index += length as usize;
        String::from_utf8(data.to_vec()).unwrap()
    };

    let server_port = ((buffer[index] as u16) << 8) | buffer[index + 1] as u16;
    index += 2;

    let (next_state, _) = read_varint(index, &buffer);

    debug!(
        "Handshake packet recieved: ({}) {{
    protocol version = {}
    server address = {}
    server port = {}
    next state = {}
}}",
        id, protocol_version, server_address, server_port, next_state
    );

    let forwards = FORWARDS_DB.borrow_data().unwrap();
    let forward = forwards
        .iter()
        .find(|forward| forward.hostname == server_address);

    if forward.is_none() {
        debug!("No forward found closing connection.");
        return;
    }
    let forward = forward.unwrap();

    debug!("Forward found {} -> {}", forward.hostname, forward.target);

    let mut server = TcpStream::connect(&forward.target).unwrap();
    server.write_all(&buffer).unwrap();
    server.write_all(client.buffer()).unwrap();

    let mut client_read = client.into_inner();
    let mut client_write = client_read.try_clone().unwrap();

    let mut server_read = server;
    let mut server_write = server_read.try_clone().unwrap();

    let c2s_connected = Arc::new(RwLock::new(true));
    let s2c_connected = c2s_connected.clone();

    // c -> s
    thread::spawn(move || {
        let mut buffer = vec![0; 128];
        while *c2s_connected.read().unwrap() {
            let length = client_read.read(&mut buffer).unwrap();

            if length > 0 {
                server_write
                    .write_all(buffer.get(0..length).unwrap())
                    .unwrap();
            } else {
                info!("Client({}) closed connection to router.", client_address);
                *c2s_connected.write().unwrap() = false;
            }
        }
    });

    // s -> c
    thread::spawn(move || {
        let mut buffer = vec![0; 128];
        while *s2c_connected.read().unwrap() {
            let length = server_read.read(&mut buffer).unwrap();

            if length > 0 {
                client_write
                    .write_all(buffer.get(0..length).unwrap())
                    .unwrap();
            } else {
                info!(
                    "Server(client: {}) closed connection to router.",
                    client_address
                );
                *s2c_connected.write().unwrap() = false;
            }
        }
    });
}
