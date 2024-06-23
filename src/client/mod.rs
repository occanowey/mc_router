use std::{
    io::{self, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    thread,
};

use mcproto::{
    self, handshake, role,
    sio::{self, StdIoConnection},
    state,
    versions::latest::{
        packets::{login, status},
        states,
    },
};
use tracing::{debug, error, field, info, info_span, trace};

use crate::{
    config::{Action, ForwardAction, Hostname, LoginAction, ServerAddr, StatusAction},
    CONFIG,
};

pub fn spawn_client_handler(stream: TcpStream, addr: SocketAddr) {
    thread::Builder::new()
        .name(format!("client({addr})"))
        .spawn(move || {
            let span = info_span!("client", %addr, username = field::Empty);
            let _enter = span.enter();

            match handle_client(stream, addr) {
                Ok(_) => {
                    info!("Connection closed");
                }
                Err(err) => match err.downcast_ref::<mcproto::error::Error>() {
                    Some(mcproto::error::Error::StreamShutdown) => {
                        info!("Connection closed");
                    }
                    Some(mcproto::error::Error::UnexpectedDisconect(err)) => {
                        info!("Connection closed: {}", err.kind());
                    }
                    _other => {
                        error!(%err, "Error while handling connection");
                    }
                },
            }
        })
        .unwrap();
}

fn handle_client(stream: TcpStream, addr: SocketAddr) -> color_eyre::Result<()> {
    debug!("Accepted connection");

    let mut sioc = sio::accept_stdio_stream::<role::Server, handshake::HandshakingState>(stream)?;

    let handshake: handshake::Handshake = sioc.expect_next_packet()?;
    trace!(?handshake, "Recieved handshake packet");
    info!("New client has connected");

    debug!("Finding action for {}", handshake.server_address);
    let action = match find_action(&handshake.server_address.parse().unwrap()) {
        Some(action) => action,
        None => {
            info!("No action found for {}", handshake.server_address);
            sioc.shutdown(Shutdown::Both)?;
            return Ok(());
        }
    };
    debug!(hostname = %handshake.server_address, ?action, "Found action");

    match handshake.next_state {
        handshake::NextState::Status => {
            let mut sioc = sioc.next_state::<states::StatusState>();
            debug!("State changed to status");

            match action.get_status_action() {
                StatusAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let version_name = r#static.version_name.unwrap_or("router".into());
                    let protocol_version = r#static
                        .protocol_version
                        .unwrap_or(handshake.protocol_version);
                    let cur_players = r#static.cur_players.unwrap_or(0);
                    let max_players = r#static.max_players.unwrap_or(20);
                    #[allow(clippy::or_fun_call)]
                    let description = r#static.description.unwrap_or("A Minecraft Server".into());

                    let request: status::c2s::StatusRequest = sioc.expect_next_packet()?;
                    trace!(?request, "Recieved request packet");

                    info!("Sending status");
                    sioc.write_packet(status::s2c::StatusResponse {
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
                    let ping: status::c2s::PingRequest = sioc.expect_next_packet()?;
                    trace!(?ping, "Recieved ping packet");
                    sioc.write_packet(status::s2c::PingResponse {
                        payload: ping.payload,
                    })?;

                    trace!("Closing connection");
                    sioc.shutdown(Shutdown::Both)?;
                }
                StatusAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("Forwarding status to {target}");
                    handle_forward_action(sioc, addr, handshake, None, target)?;
                } // StatusAction::Modify { modify: _ } => todo!(),
            }
        }
        handshake::NextState::Login => {
            let mut sioc = sioc.next_state::<states::LoginState>();
            debug!("State changed to login");
            let login_start: login::c2s::LoginStart = sioc.expect_next_packet()?;
            tracing::Span::current().record("username", &login_start.username);
            trace!(?login_start, "Recieved login start packet");

            match action.get_login_action() {
                LoginAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let kick_message = r#static.kick_message.unwrap_or("Disconnected".into());

                    info!("Sending disconnect");
                    sioc.write_packet(login::s2c::Disconnect {
                        reason: format!(r#"{{"text": "{}"}}"#, kick_message),
                    })?;

                    trace!("Closing connection");
                    sioc.shutdown(Shutdown::Both)?;
                }
                LoginAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("forwarding login to {target}");
                    handle_forward_action(sioc, addr, handshake, Some(login_start), target)?;
                }
            }
        }

        handshake::NextState::Transfer => {
            todo!("transfer state");
        }

        handshake::NextState::Unknown(other) => {
            unreachable!("unknown next state: {}", other)
        }
    }

    Ok(())
}

fn handle_forward_action<State: state::ProtocolState>(
    client: StdIoConnection<role::Server, State>,
    addr: SocketAddr,
    handshake: handshake::Handshake,
    login_start: Option<login::c2s::LoginStart>,
    target: ServerAddr,
) -> color_eyre::Result<()> {
    // todo log
    debug!("Connecting to {:?}", target);
    let mut server =
        sio::connect_stdio_stream::<_, role::Client, handshake::HandshakingState>(&target)?;

    // TODO: add config option to re write handshake to include target hostname/port
    server.write_packet(handshake)?;
    let (server_bytes, mut server) = if let Some(login_start) = login_start {
        let mut server = server.next_state::<states::LoginState>();
        server.write_packet(login_start)?;
        server.into_bytes_stream()
    } else {
        server.into_bytes_stream()
    };

    let (client_bytes, mut client) = client.into_bytes_stream();

    client.write_all(&server_bytes)?;
    server.write_all(&client_bytes)?;

    blocking_proxy(&addr, client, server)
}

fn find_action(hostname: &Hostname) -> Option<Action> {
    let config = CONFIG.read().unwrap();

    config
        .hosts
        .get(hostname)
        .or_else(|| config.get_default_host())
        .map(|host| &host.action)
        .cloned()
}

fn blocking_proxy(
    client_addr: &SocketAddr,
    client_stream: TcpStream,
    server: TcpStream,
) -> color_eyre::Result<()> {
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
) -> color_eyre::Result<thread::JoinHandle<()>> {
    Ok(thread::Builder::new().name(name).spawn(move || {
        // todo: don't ignore all errors we recieve
        let _ = io::copy(&mut from, &mut to);
        let _ = to.shutdown(Shutdown::Both);
    })?)
}
