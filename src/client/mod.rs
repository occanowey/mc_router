mod error;

use crate::FORWARDS_DB;
use error::ClientError;
use io::{BufReader, Read, Write};
use log::{debug, error, info};
use std::{
    io,
    net::TcpStream,
    sync::{Arc, RwLock},
    thread,
};

pub fn spawn_client_handler(stream: TcpStream) {
    thread::Builder::new()
        .name(format!("client({})", stream.peer_addr().unwrap()))
        .spawn(move || {
            if let Err(err) = handle_client(stream) {
                error!("{}", err);
            };
        })
        .unwrap();
}

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

fn handle_client(client: TcpStream) -> Result<(), ClientError> {
    let client_address = client.peer_addr()?;
    info!("New connection from {}", client_address);

    let mut client = BufReader::new(client);

    let mut index = 0;
    // 7 because it's the minimum length of a handshake packet but more than a varint :)
    let mut buffer = vec![0u8; 7];
    client.read_exact(&mut buffer)?;

    // todo: handle legacy ping

    let (length, length_length) = read_varint(index, &buffer);
    index += length_length;

    // length of the packet - what's already in the buffer
    let chunk_length = (length as usize) - (buffer.len() - length_length);
    let mut chunk = vec![0; chunk_length];
    client.read_exact(&mut chunk)?;
    buffer.append(&mut chunk);

    let (id, id_length) = read_varint(index, &buffer);
    index += id_length;

    if id != 0 {
        return Err(ClientError::InvalidHandshake("invalid packet id"));
    }

    let (protocol_version, protocol_version_length) = read_varint(index, &buffer);
    index += protocol_version_length;

    // messy
    let (server_address, fml) = {
        let (length, length_length) = read_varint(index, &buffer);
        index += length_length;
        let data = &buffer[index..index + length as usize];
        index += length as usize;

        let address = String::from_utf8(data.to_vec()).unwrap();
        let fml = address.ends_with("\0FML\0");

        (address.replace("\0FML\0", ""), fml)
    };

    let server_port = ((buffer[index] as u16) << 8) | buffer[index + 1] as u16;
    index += 2;

    let (next_state, _) = read_varint(index, &buffer);

    debug!(
        "Handshake packet recieved: ({}) {{
    protocol version = {}
    server address = {:?}
    server port = {}
    next state = {}
    fml = {}
}}",
        id, protocol_version, server_address, server_port, next_state, fml
    );

    let forwards = FORWARDS_DB.borrow_data().unwrap();
    let forward = forwards
        .iter()
        .find(|forward| forward.hostname == server_address);

    if forward.is_none() {
        debug!("No forward found closing connection.");
        return Ok(());
    }
    let forward = forward.unwrap();

    debug!("Forward found {} -> {}", forward.hostname, forward.target);

    let mut server = TcpStream::connect(&forward.target)?;
    server.write_all(&buffer)?;
    server.write_all(client.buffer())?;

    let mut client_read = client.into_inner();
    let mut client_write = client_read.try_clone()?;

    let mut server_read = server;
    let mut server_write = server_read.try_clone()?;

    let c2s_connected = Arc::new(RwLock::new(true));
    let s2c_connected = c2s_connected.clone();

    // c -> s
    thread::Builder::new()
        .name(format!("client({}) c->s", client_address))
        .spawn(move || {
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
        })?;

    // s -> c
    thread::Builder::new()
        .name(format!("client({}) s->c", client_address))
        .spawn(move || {
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
        })?;

    Ok(())
}
