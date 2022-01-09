use std::{
    io::{self, ErrorKind},
    net::{Shutdown, TcpStream},
    thread,
};

use log::{error, info, trace};

mod error;
mod net;

use crate::{
    config::{HostTarget, ServerAddr},
    CONFIG,
};
use error::ClientError;
use mcproto::packet::{Handshake, LoginStart, PacketWrite, Ping, Pong, Request, Response};

pub fn spawn_client_handler(stream: TcpStream) {
    thread::Builder::new()
        .name(format!("client({})", stream.peer_addr().map_or("not connected".to_owned(), |a| a.to_string())))
        .spawn(move || {
            if let Err(err) = handle_client(stream) {
                error!("{}", err);
            };
        })
        .unwrap();
}

fn handle_client(stream: TcpStream) -> Result<(), ClientError> {
    let mut client = net::handler_from_stream(stream);

    trace!("reading handshake from {}", &client.address);
    let handshake = match client.read::<Handshake>() {
        Err(ClientError::IO(ioerr)) if ioerr.kind() == ErrorKind::UnexpectedEof => {
            trace!("client didn't send any data, closing");
            return Ok(());
        }

        handshake => handshake,
    }?;
    trace!("read handshake: {:?}", handshake);

    info!("New connection from {}", client.address);

    trace!(
        "finding target for hostname: {:?}",
        handshake.server_address
    );
    let target = match find_target(&handshake.server_address) {
        Some(target) => target,
        None => {
            info!(
                "Could not find target for {:?} requested by {}",
                handshake.server_address, client.address
            );

            client.close()?;
            return Ok(());
        }
    };
    trace!("found target: {:?}", &target);

    match handshake.next_state {
        1 => {
            let mut client = client.status();
            match target {
                HostTarget::Status {
                    online_players,
                    max_players,
                    description,
                } => {
                    info!("Sending status: {}", client.address);
                    let _ = client.read::<Request>()?;
                    client.write(Response {
                        // TODO: have serde do this for me
                        response: format!(
                            r#"{{
                                "version": {{
                                    "name": "router",
                                    "protocol": {}
                                }},
                                "players": {{
                                    "max": {},
                                    "online": {}
                                }},
                                "description": {{
                                    "text": "{}"
                                }}
                            }}"#,
                            handshake.protocol_version, max_players, online_players, description
                        ),
                    })?;

                    let ping = client.read::<Ping>()?;
                    client.write(Pong { data: ping.data })?;

                    client.close()?;
                }

                HostTarget::Forward(target) => {
                    info!("Forwarding status: {} -> {}", client.address, &target);

                    let mut server = connect_to_server(&target)?;

                    // TODO: add config option to re write handshake to include target hostname/port
                    handshake.write(&mut server)?;
                    blocking_proxy(&client.address, client.stream, server)?;

                    info!("Disconnected {}", client.address);
                }
            }
        }
        2 => {
            let mut client = client.login();
            let login_start = client.read::<LoginStart>()?;

            match target {
                HostTarget::Status { .. } => {
                    // TODO: figure out what to do here...
                    // rename status to static and have a kick msg?

                    info!(
                        "Client tried to login to status target {} ({})",
                        client.address, login_start.username
                    );
                    client.close()?;
                }

                HostTarget::Forward(target) => {
                    info!(
                        "Forwarding login: {} ({}) -> {}",
                        client.address, login_start.username, target
                    );

                    let mut server = connect_to_server(&target)?;

                    // TODO: add config option to re write handshake to include target hostname/port
                    handshake.write(&mut server)?;
                    login_start.write(&mut server)?;
                    blocking_proxy(&client.address, client.stream, server)?;

                    info!("Disconnected {} ({})", client.address, login_start.username);
                }
            }
        }

        other => unreachable!("next state should be 1 or 2, got {}", other),
    }

    Ok(())
}

fn find_target(hostname: &str) -> Option<HostTarget> {
    let config = CONFIG.read().unwrap();

    config
        .virtualhosts
        .iter()
        .find(|f| f.hostname == hostname)
        .or_else(|| config.get_default_target())
        .map(|h| &h.target)
        .cloned()
}

fn connect_to_server(address: &ServerAddr) -> Result<TcpStream, ClientError> {
    trace!("connecting to {:?}", address);
    Ok(match TcpStream::connect(&address) {
        // don't really remember why this was a thing

        // Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
        //     info!("Could not connect to server, closing client connection.");
        //     return Ok(());
        // }

        res => res,
    }?)
}

fn blocking_proxy(
    client_address: &str,
    client_stream: TcpStream,
    server: TcpStream,
) -> Result<(), ClientError> {
    let client_read = client_stream;
    let client_write = client_read.try_clone()?;

    let server_read = server;
    let server_write = server_read.try_clone()?;

    let cs_thread = spawn_copy_thread(
        format!("client({}) c->s", client_address),
        client_read,
        server_write,
    )?;
    let sc_thread = spawn_copy_thread(
        format!("client({}) s->c", client_address),
        server_read,
        client_write,
    )?;

    cs_thread.join().unwrap();
    sc_thread.join().unwrap();

    Ok(())
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
