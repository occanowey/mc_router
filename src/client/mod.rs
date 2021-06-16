mod error;

use crate::{config::Forward, read_types::ReadMCTypesExt, util::CachedReader, CONFIG};
use error::ClientError;
use io::{Read, Write};
use log::{error, info, trace};
use std::{
    io,
    net::{Shutdown, TcpStream},
    thread,
};

trait ClientState {}

struct Client<S: ClientState> {
    stream: CachedReader<TcpStream>,
    address: String,

    extra: S,
}

struct Initialize;
struct PostHandshake {
    handshake: Handshake,
    forward: Forward,
}

enum NextState {
    Status,
    Login { username: String },
}

struct Proxy {
    forward: Forward,
    next_state: NextState,
}

impl ClientState for Initialize {}
impl ClientState for PostHandshake {}
impl ClientState for Proxy {}

enum ClientStatus<S: ClientState> {
    Open(Client<S>),
    Closed(String),
}

impl Client<Initialize> {
    fn new(stream: TcpStream) -> Result<Client<Initialize>, ClientError> {
        let address = stream.peer_addr()?.to_string();
        let stream = CachedReader::new(stream);

        Ok(Client {
            stream,
            address,

            extra: Initialize,
        })
    }

    fn close<S: ClientState>(self) -> Result<ClientStatus<S>, ClientError> {
        self.stream.into_inner().shutdown(Shutdown::Both)?;

        Ok(ClientStatus::Closed(self.address))
    }

    fn handshake(mut self) -> Result<ClientStatus<PostHandshake>, ClientError> {
        trace!("reading handshake from {}", self.address);
        let handshake = match decode_handshake(&mut self.stream) {
            Err(ClientError::IO(ioerr)) if ioerr.kind() == io::ErrorKind::UnexpectedEof => {
                trace!("client didn't send any data, closing");
                return self.close();
            }

            handshake => handshake,
        }?;
        trace!("read handshake: {:?}", handshake);

        info!("New connection from {}", self.address);

        trace!(
            "finding forward for hostname: {:?}",
            handshake.server_address
        );
        let forward = {
            let config = CONFIG.read().unwrap();

            config
                .forwards
                .iter()
                .find(|f| f.hostname == &handshake.server_address)
                .map(|f| f.clone())
        };

        match forward {
            Some(forward) => {
                trace!("found forward: {:?}", &forward);
                Ok(ClientStatus::Open(Client {
                    stream: self.stream,
                    address: self.address,
                    extra: PostHandshake {
                        handshake,
                        forward,
                    },
                }))
            }

            None => {
                info!(
                    "Could not find forward for {:?} requested by {}",
                    handshake.server_address, self.address
                );
                self.close()
            }
        }
    }
}

impl Client<PostHandshake> {
    fn prepare_proxy(mut self) -> Result<Client<Proxy>, ClientError> {
        let next_state = match self.extra.handshake.next_state {
            1 => NextState::Status,
            2 => {
                let _length = self.stream.read_varint()?;

                let (id, _) = self.stream.read_varint()?;
                if id != 0 {
                    return Err(ClientError::InvalidHandshake("invalid packet id"));
                }

                let (username, _) = self.stream.read_string()?;

                NextState::Login { username }
            }

            _ => unreachable!("next state should be 1 or 2"),
        };

        Ok(Client {
            stream: self.stream,
            address: self.address,
            extra: Proxy {
                forward: self.extra.forward,
                next_state,
            },
        })
    }
}

impl Client<Proxy> {
    fn proxy(self) -> Result<(), ClientError> {
        use NextState::*;

        let forward = self.extra.forward;
        let state = self.extra.next_state;

        match state {
            Status => info!("Forwarding status: {} -> {}", self.address, forward.target),
            Login { ref username } => info!(
                "Forwarding login: {} ({}) -> {}",
                self.address, username, forward.target
            ),
        }

        trace!("connecting to {:?}", forward.target);
        let mut server = match TcpStream::connect(&forward.target) {
            Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                info!("Could not connect to server, closing client connection.");
                return Ok(());
            }

            res => res,
        }?;

        // TODO: add config option to re write handshake to include target hostname/port
        server.write_all(self.stream.cache())?;

        let client_read = self.stream.into_inner();
        let client_write = client_read.try_clone()?;

        let server_read = server;
        let server_write = server_read.try_clone()?;

        let cs_thread = spawn_copy_thread(
            format!("client({}) c->s", self.address),
            client_read,
            server_write,
        )?;
        let sc_thread = spawn_copy_thread(
            format!("client({}) s->c", self.address),
            server_read,
            client_write,
        )?;

        cs_thread.join().unwrap();
        sc_thread.join().unwrap();

        info!(
            "Disconnecting {}",
            match state {
                Login { ref username } => format!("{} ({})", self.address, username),
                Status => self.address,
            }
        );

        Ok(())
    }
}

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

fn handle_client(stream: TcpStream) -> Result<(), ClientError> {
    match Client::new(stream)?.handshake()? {
        ClientStatus::Open(client) => client.prepare_proxy()?.proxy(),
        ClientStatus::Closed(_) => Ok(()),
    }
}

fn spawn_copy_thread(
    name: String,
    mut from: TcpStream,
    mut to: TcpStream,
) -> Result<thread::JoinHandle<()>, io::Error> {
    thread::Builder::new().name(name).spawn(move || {
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
