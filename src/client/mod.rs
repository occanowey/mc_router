mod error;

use crate::{read_types::ReadMCTypesExt, util::CachedReader, CONFIG};
use error::ClientError;
use io::{Read, Write};
use log::{debug, error, info, trace};
use std::{io, net::{Shutdown, TcpStream}, thread};

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
    trace!("New connection from {}", client_address);

    let mut client = CachedReader::new(client);

    let handshake = match decode_handshake(&mut client) {
        Err(ClientError::IO(ioerr)) if ioerr.kind() == io::ErrorKind::UnexpectedEof => {
            return Ok(())
        },

        handshake => handshake,
    }?;

    info!("Client connected from {}", client_address);
    trace!("Handshake packet recieved: {:?}", handshake);

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

    let client_read = client.into_inner();
    let client_write = client_read.try_clone()?;

    let server_read = server;
    let server_write = server_read.try_clone()?;

    let cs_thread = spawn_copy_thread(format!("client({}) c->s", client_address), client_read, server_write)?;
    let sc_thread = spawn_copy_thread(format!("client({}) s->c", client_address), server_read, client_write)?;

    cs_thread.join().unwrap();
    sc_thread.join().unwrap();

    info!("Disconnecting client {}", client_address);

    Ok(())
}

fn spawn_copy_thread(name: String, mut from: TcpStream, mut to: TcpStream) -> Result<thread::JoinHandle<()>, io::Error> {
    thread::Builder::new()
        .name(name)
        .spawn(move || {
            // Ignore all errors we recieve
            let _ = io::copy(&mut from, &mut to);
            let _ = to.shutdown(Shutdown::Both);
        })
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
