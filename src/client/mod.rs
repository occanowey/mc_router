use std::{
    io,
    net::{Shutdown, SocketAddr, TcpStream},
    thread,
};

use mcproto::{
    self, handshake, role,
    sio::{self, StdIoConnection},
};
use multi_version::Protocol;
use tracing::{debug, error, field, info, info_span, trace, warn};

use crate::{
    config::{Action, ForwardAction, Hostname, LoginAction, StatusAction},
    CONFIG,
};

mod multi_version;
mod version_impls;

pub fn spawn_client_handler(stream: TcpStream, addr: SocketAddr) {
    thread::Builder::new()
        .name(format!("client({addr})"))
        .spawn(move || {
            let span = info_span!("client", %addr, username = field::Empty);
            let _enter = span.enter();

            match handshake_client(stream, addr) {
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

fn handshake_client(stream: TcpStream, addr: SocketAddr) -> color_eyre::Result<()> {
    debug!("Accepted connection");

    let mut sioc = sio::accept_stdio_stream::<role::Server, handshake::HandshakingState>(stream)?;

    let handshake: handshake::Handshake = sioc.expect_next_packet()?;
    trace!(?handshake, "Recieved handshake packet");
    info!("New client has connected");

    match handshake.protocol_version {
        // 3 => handle_client::<version_impls::ProtocolV3>(sioc, handshake, addr),
        4 => handle_client::<version_impls::ProtocolV4>(sioc, handshake, addr),
        5 => handle_client::<version_impls::ProtocolV5>(sioc, handshake, addr),
        47 => handle_client::<version_impls::ProtocolV47>(sioc, handshake, addr),
        107 => handle_client::<version_impls::ProtocolV107>(sioc, handshake, addr),
        108 => handle_client::<version_impls::ProtocolV108>(sioc, handshake, addr),
        109 => handle_client::<version_impls::ProtocolV109>(sioc, handshake, addr),
        110 => handle_client::<version_impls::ProtocolV110>(sioc, handshake, addr),
        210 => handle_client::<version_impls::ProtocolV210>(sioc, handshake, addr),
        315 => handle_client::<version_impls::ProtocolV315>(sioc, handshake, addr),
        316 => handle_client::<version_impls::ProtocolV316>(sioc, handshake, addr),
        335 => handle_client::<version_impls::ProtocolV335>(sioc, handshake, addr),
        338 => handle_client::<version_impls::ProtocolV338>(sioc, handshake, addr),
        340 => handle_client::<version_impls::ProtocolV340>(sioc, handshake, addr),
        393 => handle_client::<version_impls::ProtocolV393>(sioc, handshake, addr),
        401 => handle_client::<version_impls::ProtocolV401>(sioc, handshake, addr),
        404 => handle_client::<version_impls::ProtocolV404>(sioc, handshake, addr),
        477 => handle_client::<version_impls::ProtocolV477>(sioc, handshake, addr),
        480 => handle_client::<version_impls::ProtocolV480>(sioc, handshake, addr),
        485 => handle_client::<version_impls::ProtocolV485>(sioc, handshake, addr),
        490 => handle_client::<version_impls::ProtocolV490>(sioc, handshake, addr),
        498 => handle_client::<version_impls::ProtocolV498>(sioc, handshake, addr),
        573 => handle_client::<version_impls::ProtocolV573>(sioc, handshake, addr),
        575 => handle_client::<version_impls::ProtocolV575>(sioc, handshake, addr),
        578 => handle_client::<version_impls::ProtocolV578>(sioc, handshake, addr),
        735 => handle_client::<version_impls::ProtocolV735>(sioc, handshake, addr),
        736 => handle_client::<version_impls::ProtocolV736>(sioc, handshake, addr),
        751 => handle_client::<version_impls::ProtocolV751>(sioc, handshake, addr),
        753 => handle_client::<version_impls::ProtocolV753>(sioc, handshake, addr),
        754 => handle_client::<version_impls::ProtocolV754>(sioc, handshake, addr),
        755 => handle_client::<version_impls::ProtocolV755>(sioc, handshake, addr),
        756 => handle_client::<version_impls::ProtocolV756>(sioc, handshake, addr),
        757 => handle_client::<version_impls::ProtocolV757>(sioc, handshake, addr),
        758 => handle_client::<version_impls::ProtocolV758>(sioc, handshake, addr),
        759 => handle_client::<version_impls::ProtocolV759>(sioc, handshake, addr),
        760 => handle_client::<version_impls::ProtocolV760>(sioc, handshake, addr),
        761 => handle_client::<version_impls::ProtocolV761>(sioc, handshake, addr),
        762 => handle_client::<version_impls::ProtocolV762>(sioc, handshake, addr),
        763 => handle_client::<version_impls::ProtocolV763>(sioc, handshake, addr),
        764 => handle_client::<version_impls::ProtocolV764>(sioc, handshake, addr),
        765 => handle_client::<version_impls::ProtocolV765>(sioc, handshake, addr),
        766 => handle_client::<version_impls::ProtocolV766>(sioc, handshake, addr),
        767 => handle_client::<version_impls::ProtocolV767>(sioc, handshake, addr),

        other => {
            warn!("unknown protocol version: {}, defaulting to latest.", other);

            handle_client::<version_impls::ProtocolV767>(sioc, handshake, addr)
        }
    }
}

fn handle_client<P: Protocol>(
    connection: StdIoConnection<role::Server, handshake::HandshakingState>,
    handshake: handshake::Handshake,
    addr: SocketAddr,
) -> color_eyre::Result<()>
where
    <P::StatusState as mcproto::state::RoleStatePackets<mcproto::role::Server>>::RecvPacket:
        mcproto::packet::PacketFromIdBody,
    <P::LoginState as mcproto::state::RoleStatePackets<mcproto::role::Server>>::RecvPacket:
        mcproto::packet::PacketFromIdBody,
{
    debug!("Finding action for {}", handshake.server_address);
    let action = match find_action(&handshake.server_address.parse().unwrap()) {
        Some(action) => action,
        None => {
            info!("No action found for {}", handshake.server_address);
            connection.shutdown(Shutdown::Both)?;
            return Ok(());
        }
    };
    debug!(hostname = %handshake.server_address, ?action, "Found action");

    match handshake.next_state {
        handshake::NextState::Status => {
            let mut connection = P::status_state(connection);
            debug!("State changed to status");

            match action.get_status_action() {
                StatusAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let version_name = r#static.version_name.unwrap_or("router".into());
                    let protocol_version = r#static
                        .protocol_version
                        .unwrap_or(handshake.protocol_version);
                    let online_players = r#static.cur_players.unwrap_or(0);
                    let max_players = r#static.max_players.unwrap_or(20);
                    #[allow(clippy::or_fun_call)]
                    let description = r#static.description.unwrap_or("A Minecraft Server".into());

                    let request = P::read_status_request(&mut connection)?;
                    trace!(?request, "Recieved request packet");

                    info!("Sending status");
                    P::write_status_response(
                        &mut connection,
                        multi_version::StatusResponse {
                            version_name: version_name.clone(),
                            protocol_version,
                            max_players,
                            online_players,
                            description: description.clone(),
                        },
                    )?;

                    // attempt ping/pong
                    let ping = P::read_ping_request(&mut connection)?;
                    trace!(?ping, "Recieved ping packet");
                    P::write_ping_response(
                        &mut connection,
                        multi_version::PingResponse {
                            payload: ping.payload,
                        },
                    )?;

                    trace!("Closing connection");
                    connection.shutdown(Shutdown::Both)?;
                }
                StatusAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("Forwarding status to {target}");
                    P::forward_status(connection, addr, handshake, target)?;
                } // StatusAction::Modify { modify: _ } => todo!(),
            }
        }
        handshake::NextState::Login => {
            let mut connection = P::login_state(connection);
            debug!("State changed to login");
            let login_start = P::read_login_start(&mut connection)?;
            tracing::Span::current().record("username", &login_start.username);
            trace!(?login_start, "Recieved login start packet");

            match action.get_login_action() {
                LoginAction::Static { r#static } => {
                    #[allow(clippy::or_fun_call)]
                    let kick_message = r#static.kick_message.unwrap_or("Disconnected".into());

                    info!("Sending disconnect");
                    P::write_disconnect(
                        &mut connection,
                        multi_version::Disconnect {
                            reason: kick_message,
                        },
                    )?;

                    trace!("Closing connection");
                    connection.shutdown(Shutdown::Both)?;
                }
                LoginAction::Forward {
                    forward: ForwardAction(target),
                } => {
                    info!("forwarding login to {target}");
                    P::forward_login(connection, addr, handshake, login_start, target)?;
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
