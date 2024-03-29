use std::{
    io,
    net::{Shutdown, SocketAddr, TcpStream},
    thread,
};

use mcproto::{
    net::{handler_from_stream, side::Server, state::NetworkState},
    packet::{
        handshaking::{Handshake, NextState},
        login::{Disconnect, LoginStart},
        status::{Ping, Pong, Request, Response},
        PacketWrite,
    },
};
use tracing::{debug, error, field, info, info_span, trace};

mod error;

use crate::{
    config::{Action, ForwardAction, LoginAction, ServerAddr, StatusAction},
    CONFIG,
};
use error::ClientError;

pub type NetworkHandler<S> = mcproto::net::NetworkHandler<Server, S>;

pub fn spawn_client_handler(stream: TcpStream, addr: SocketAddr) {
    thread::Builder::new()
        .name(format!("client({addr})"))
        .spawn(move || {
            let span = info_span!("client", %addr, username = field::Empty);
            let _enter = span.enter();

            match handle_client(stream, addr) {
                Ok(_) | Err(ClientError::Proto(mcproto::error::Error::UnexpectedDisconect(_))) => {
                    info!("Connection closed");
                }
                Err(err) => {
                    error!(%err, "Error while handling connection");
                }
            }
        })
        .unwrap();
}

fn handle_client(stream: TcpStream, addr: SocketAddr) -> Result<(), ClientError> {
    debug!("Accepted connection");

    stream.set_nodelay(true)?;
    let mut client = handler_from_stream(stream)?;

    let handshake = client.read::<Handshake>()?;
    trace!(?handshake, "Recieved handshake packet");
    info!("New client has connected");

    debug!("Finding action for {}", handshake.server_address);
    let action = match find_action(&handshake.server_address) {
        Some(action) => action,
        None => {
            info!("No action found for {}", handshake.server_address);
            client.close()?;
            return Ok(());
        }
    };
    debug!(hostname = %handshake.server_address, ?action, "Found action");

    match handshake.next_state {
        NextState::Status => {
            let mut client = client.status();
            debug!("State changed to status");

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

                    let request = client.read::<Request>()?;
                    trace!(?request, "Recieved request packet");

                    info!("Sending status");
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
                    let ping: Ping = client.read()?;
                    trace!(?ping, "Recieved ping packet");
                    client.write(Pong { data: ping.data })?;

                    trace!("Closing connection");
                    client.close()?;
                }
                StatusAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("Forwarding status to {target}");
                    handle_forward_action(client, addr, &handshake, None, target)?;
                }
                StatusAction::Modify { modify: _ } => todo!(),
            }
        }
        NextState::Login => {
            let mut client = client.login();
            debug!("State changed to login");
            let login_start = client.read::<LoginStart>()?;
            tracing::Span::current().record("username", &login_start.username);
            trace!(?login_start, "Recieved login start packet");

            match action.get_login_action() {
                LoginAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let kick_message = r#static.kick_message.unwrap_or("Disconnected".into());

                    info!("Sending disconnect");
                    client.write(Disconnect {
                        reason: format!(r#"{{"text": "{}"}}"#, kick_message),
                    })?;

                    trace!("Closing connection");
                    client.close()?;
                }
                LoginAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("forwarding login to {target}");
                    handle_forward_action(client, addr, &handshake, Some(&login_start), target)?;
                }
            }
        }

        NextState::Unknown(other) => unreachable!("unknown next state: {}", other),
    }

    Ok(())
}

fn handle_forward_action<S: NetworkState>(
    client: NetworkHandler<S>,
    addr: SocketAddr,
    handshake: &Handshake,
    login_start: Option<&LoginStart>,
    target: ServerAddr,
) -> Result<(), ClientError> {
    // todo log
    let mut server = connect_to_server(&target)?;

    // TODO: add config option to re write handshake to include target hostname/port
    handshake.write(&mut server)?;
    if let Some(login_start) = login_start {
        login_start.write(&mut server)?;
    }

    blocking_proxy(&addr, client.into_stream(), server)
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

fn connect_to_server(addr: &ServerAddr) -> Result<TcpStream, ClientError> {
    debug!("Connecting to {:?}", addr);
    Ok(match TcpStream::connect(&addr) {
        // don't really remember why this was a thing

        // Err(ref e) if e.kind() == io::ErrorKind::ConnectionRefused => {
        //     info!("Could not connect to server, closing client connection.");
        //     return Ok(());
        // }
        //
        res => res,
    }?)
}

fn blocking_proxy(
    client_addr: &SocketAddr,
    client_stream: TcpStream,
    server: TcpStream,
) -> Result<(), ClientError> {
    let client_read = client_stream;
    let client_write = client_read.try_clone()?;

    let server_read = server;
    let server_write = server_read.try_clone()?;

    let cs_thread = spawn_copy_thread(
        format!("client({}) c->s", client_addr),
        client_read,
        server_write,
    )?;
    let sc_thread = spawn_copy_thread(
        format!("client({}) s->c", client_addr),
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
