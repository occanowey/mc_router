mod error;

use crate::{read_types::ReadMCTypesExt, util::CachedReader, CONFIG};
use error::ClientError;
use io::{Read, Write};
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

fn handle_client(client: TcpStream) -> Result<(), ClientError> {
    let client_address = client.peer_addr()?;
    info!("New connection from {}", client_address);

    let mut client = CachedReader::new(client);

    let handshake = decode_handshake(&mut client)?;
    debug!("Handshake packet recieved: {:?}", handshake);

    let forward = {
        let config = CONFIG.read().unwrap();
        let forward = config
            .forwards
            .iter()
            .find(|forward| forward.hostname == &handshake.server_address);

        if forward.is_none() {
            debug!("No forward found closing connection.");
            return Ok(());
        }

        forward.unwrap().clone()
    };

    debug!("Forward found {} -> {}", forward.hostname, forward.target);

    let mut server = match TcpStream::connect(&forward.target) {
        Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
            info!("Could not connect to server, closing client connection.");
            return Ok(());
        }

        res => res,
    }?;

    // TODO: add config option to re write handshake to include target hostname/port
    server.write_all(client.cache())?;

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

#[derive(Debug)]
struct Handshake {
    protocol_version: i32,
    server_address: String,
    server_port: u16,
    next_state: i32,

    fml: bool,
}

fn decode_handshake<R: Read>(reader: &mut R) -> Result<Handshake, ClientError> {
    // todo: maybe handle legacy ping?

    let _length = reader.read_varint()?;

    let (id, _) = reader.read_varint()?;
    if id != 0 {
        return Err(ClientError::InvalidHandshake("invalid packet id"));
    }

    let (protocol_version, _) = reader.read_varint()?;
    let (server_address, _) = reader.read_string()?;
    let server_port = reader.read_ushort()?;
    let (next_state, _) = reader.read_varint()?;

    let fml = server_address.ends_with("\0FML\0");
    let server_address = server_address.replace("\0FML\0", "");

    Ok(Handshake {
        protocol_version,
        server_address,
        server_port,
        next_state,
        fml,
    })
}
