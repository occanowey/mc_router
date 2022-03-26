use std::{
    io::{self, ErrorKind},
    net::{Shutdown, TcpStream},
    thread,
};

use log::{error, info, trace};
use mcproto::{
    net::handler_from_stream,
    packet::{
        handshaking::{Handshake, NextState},
        login::{Disconnect, LoginStart},
        status::{Ping, Pong, Request, Response},
        PacketWrite,
    },
};

mod error;

use crate::{
    config::{Action, ForwardAction, LoginAction, ServerAddr, StatusAction},
    CONFIG,
};
use error::ClientError;

pub fn spawn_client_handler(stream: TcpStream) {
    thread::Builder::new()
        .name(format!(
            "client({})",
            stream
                .peer_addr()
                .map_or("not connected".to_owned(), |a| a.to_string())
        ))
        .spawn(move || {
            if let Err(err) = handle_client(stream) {
                error!("{}", err);
            };
        })
        .unwrap();
}

fn handle_client(stream: TcpStream) -> Result<(), ClientError> {
    let address = stream.peer_addr()?.to_string();
    let mut client = handler_from_stream(stream);

    trace!("reading handshake from {}", &address);
    let handshake = match client.read::<Handshake>() {
        Err(ioerr) if ioerr.kind() == ErrorKind::UnexpectedEof => {
            trace!("client didn't send any data, closing");
            return Ok(());
        }

        handshake => handshake,
    }?;
    trace!("read handshake: {:?}", handshake);

    info!("New connection from {}", &address);

    trace!(
        "finding action for hostname: {:?}",
        handshake.server_address
    );
    let action = match find_action(&handshake.server_address) {
        Some(action) => action,
        None => {
            info!(
                "Could not find action for {:?} requested by {}",
                handshake.server_address, &address
            );

            client.close()?;
            return Ok(());
        }
    };
    trace!("found action: {:?}", &action);

    match handshake.next_state {
        NextState::Status => {
            let mut client = client.status();

            match action.get_status_action() {
                StatusAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let version_name = r#static.version_name.unwrap_or("router".into());
                    let protocol_version = r#static
                        .protocol_version
                        .unwrap_or(*handshake.protocol_version);
                    let cur_players = r#static.cur_players.unwrap_or(0);
                    let max_players = r#static.max_players.unwrap_or(20);
                    #[allow(clippy::or_fun_call)]
                    let description = r#static.description.unwrap_or("A Minecraft Server".into());

                    info!("Sending status: {}", &address);

                    let _ = client.read::<Request>()?;
                    client.write(Response {
                        // TODO: have serde do this for me
                        response: format!(
                            r#"{{
                                "version": {{
                                    "name": "{}",
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
                            version_name, protocol_version, max_players, cur_players, description
                        ),
                    })?;

                    // attempt ping/pong
                    match client.read() {
                        // respond to ping request
                        Ok(Ping { data }) => client.write(Pong { data }),

                        // don't try to respond if stream was closed
                        Err(ioerr) if ioerr.kind() == ErrorKind::UnexpectedEof => Ok(()),

                        // bubble up other errors
                        Err(other) => Err(other),
                    }?;

                    info!("Disconnected {}", &address);
                    client.close()?;
                }
                StatusAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("Forwarding status: {} -> {}", &address, &target);

                    let mut server = connect_to_server(&target)?;

                    // TODO: add config option to re write handshake to include target hostname/port
                    handshake.write(&mut server)?;
                    blocking_proxy(&address, client.into_stream(), server)?;

                    info!("Disconnected {}", &address);
                }
                StatusAction::Modify { modify } => todo!(),
            }
        }
        NextState::Login => {
            let mut client = client.login();
            let login_start = client.read::<LoginStart>()?;

            match action.get_login_action() {
                LoginAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let kick_message = r#static.kick_message.unwrap_or("Disconnected".into());

                    info!("Sending disconnect: {}", &address);

                    client.write(Disconnect {
                        reason: format!(r#"{{"text": "{}"}}"#, kick_message),
                    })?;

                    info!("Disconnected {}", &address);
                    client.close()?;
                }
                LoginAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!(
                        "Forwarding login: {} ({}) -> {}",
                        &address, login_start.username, target
                    );

                    let mut server = connect_to_server(&target)?;

                    // TODO: add config option to re write handshake to include target hostname/port
                    handshake.write(&mut server)?;
                    login_start.write(&mut server)?;
                    blocking_proxy(&address, client.into_stream(), server)?;

                    info!("Disconnected {} ({})", &address, login_start.username);
                }
            }
        }

        NextState::Unknown(other) => unreachable!("unknown next state: {}", other),
    }

    Ok(())
}

fn find_action(hostname: &str) -> Option<Action> {
    let config = CONFIG.read().unwrap();

    config
        .virtualhosts
        .iter()
        .find(|f| f.hostname == hostname)
        .or_else(|| config.get_default_host())
        .map(|h| &h.action)
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
