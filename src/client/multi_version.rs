use std::{
    convert::{Into, TryFrom},
    io::Write,
    net::SocketAddr,
};

use mcproto::{error, handshake, packet, role, state, stdio::StdIoConnection, uuid::Uuid};
use tracing::debug;

use crate::{client::blocking_proxy, config::ServerAddr};

#[derive(Debug)]
pub struct StatusRequest;

#[derive(Debug)]
pub struct StatusResponse {
    pub version_name: String,
    pub protocol_version: i32,
    pub max_players: i64,
    pub online_players: i64,
    pub description: String,
}

impl StatusResponse {
    pub fn to_json(&self) -> String {
        // TODO: have serde do this for me
        format!(
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
            self.version_name,
            self.protocol_version,
            self.max_players,
            self.online_players,
            self.description
        )
    }
}

#[derive(Debug)]
pub struct PingRequest {
    pub payload: i64,
}

#[derive(Debug)]
pub struct PingResponse {
    pub payload: i64,
}

pub trait StatusState:
    state::ProtocolState
    + state::RoleStatePackets<role::Server>
    + state::NextProtocolState<handshake::HandshakingState>
    + Sized
where
    Self::RecvPacket: packet::PacketFromIdBody,
{
    type StatusRequest: packet::Packet
        + state::RoleStateReadPacket<role::Server, Self>
        + TryFrom<Self::RecvPacket, Error = error::Error>
        + Into<StatusRequest>;

    type StatusResponse: packet::Packet
        + state::RoleStateWritePacket<role::Server, Self>
        + From<StatusResponse>;

    type PingRequest: packet::Packet
        + state::RoleStateReadPacket<role::Server, Self>
        + TryFrom<Self::RecvPacket, Error = error::Error>
        + Into<PingRequest>;

    type PingResponse: packet::Packet
        + state::RoleStateWritePacket<role::Server, Self>
        + From<PingResponse>;
}

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
    pub uuid: Option<Uuid>,
}

#[derive(Debug)]
pub struct Disconnect {
    pub reason: String,
}

impl Disconnect {
    pub fn to_json(&self) -> String {
        format!(r#"{{"text": "{}"}}"#, self.reason)
    }
}

pub trait LoginState:
    state::ProtocolState
    + state::RoleStatePackets<role::Server>
    + state::NextProtocolState<handshake::HandshakingState>
    + Sized
where
    Self::RecvPacket: packet::PacketFromIdBody,
{
    type LoginStart: packet::Packet
        + state::RoleStateReadPacket<role::Server, Self>
        + TryFrom<Self::RecvPacket, Error = error::Error>
        + Into<LoginStart>
        + state::RoleStateWritePacket<role::Client, Self>
        + From<LoginStart>;

    type Disconnect: packet::Packet
        + state::RoleStateWritePacket<role::Server, Self>
        + From<Disconnect>;
}

pub trait Protocol
where
    <Self::StatusState as state::RoleStatePackets<role::Server>>::RecvPacket:
        packet::PacketFromIdBody,

    <Self::LoginState as state::RoleStatePackets<role::Server>>::RecvPacket:
        packet::PacketFromIdBody,
{
    const VERSION: i32;

    type StatusState: StatusState;
    type LoginState: LoginState;

    fn status_state(
        connection: StdIoConnection<role::Server, handshake::HandshakingState>,
    ) -> StdIoConnection<role::Server, Self::StatusState> {
        connection.next_state()
    }

    fn read_status_request(
        connection: &mut StdIoConnection<role::Server, Self::StatusState>,
    ) -> color_eyre::Result<StatusRequest> {
        let request: <Self::StatusState as StatusState>::StatusRequest =
            connection.expect_next_packet()?;
        Ok(request.into())
    }

    fn write_status_response(
        connection: &mut StdIoConnection<role::Server, Self::StatusState>,
        status_response: StatusResponse,
    ) -> color_eyre::Result<()> {
        connection.write_packet(
            Into::<<Self::StatusState as StatusState>::StatusResponse>::into(status_response),
        )?;
        Ok(())
    }
    fn read_ping_request(
        connection: &mut StdIoConnection<role::Server, Self::StatusState>,
    ) -> color_eyre::Result<PingRequest> {
        let request: <Self::StatusState as StatusState>::PingRequest =
            connection.expect_next_packet()?;
        Ok(request.into())
    }

    fn write_ping_response(
        connection: &mut StdIoConnection<role::Server, Self::StatusState>,
        status_response: PingResponse,
    ) -> color_eyre::Result<()> {
        connection.write_packet(
            Into::<<Self::StatusState as StatusState>::PingResponse>::into(status_response),
        )?;
        Ok(())
    }

    fn forward_status(
        connection: StdIoConnection<role::Server, Self::StatusState>,
        addr: SocketAddr,
        handshake: handshake::Handshake,
        target: ServerAddr,
    ) -> color_eyre::Result<()> {
        // todo log
        debug!("Connecting to {:?}", target);
        let mut server =
            mcproto::stdio::connect_stdio_stream::<_, role::Client, handshake::HandshakingState>(
                &target,
            )?;

        // TODO: add config option to re write handshake to include target hostname/port
        server.write_packet(handshake)?;

        let (server_bytes, mut server) = server.into_bytes_stream();
        let (client_bytes, mut client) = connection.into_bytes_stream();

        client.write_all(&server_bytes)?;
        server.write_all(&client_bytes)?;

        blocking_proxy(&addr, client, server)
    }

    fn login_state(
        connection: StdIoConnection<role::Server, handshake::HandshakingState>,
    ) -> StdIoConnection<role::Server, Self::LoginState> {
        connection.next_state()
    }

    fn read_login_start(
        connection: &mut StdIoConnection<role::Server, Self::LoginState>,
    ) -> color_eyre::Result<LoginStart> {
        let login_start: <Self::LoginState as LoginState>::LoginStart =
            connection.expect_next_packet()?;
        Ok(login_start.into())
    }

    fn write_disconnect(
        connection: &mut StdIoConnection<role::Server, Self::LoginState>,
        disconnect: Disconnect,
    ) -> color_eyre::Result<()> {
        connection.write_packet(Into::<<Self::LoginState as LoginState>::Disconnect>::into(
            disconnect,
        ))?;
        Ok(())
    }

    fn forward_login(
        connection: StdIoConnection<role::Server, Self::LoginState>,
        addr: SocketAddr,
        handshake: handshake::Handshake,
        login_start: LoginStart,
        target: ServerAddr,
    ) -> color_eyre::Result<()> {
        // todo log
        debug!("Connecting to {:?}", target);
        let mut server =
            mcproto::stdio::connect_stdio_stream::<_, role::Client, handshake::HandshakingState>(
                &target,
            )?;

        // TODO: add config option to re write handshake to include target hostname/port
        server.write_packet(handshake)?;
        let mut server = server.next_state::<Self::LoginState>();
        server.write_packet(Into::<<Self::LoginState as LoginState>::LoginStart>::into(
            login_start,
        ))?;

        let (server_bytes, mut server) = server.into_bytes_stream();
        let (client_bytes, mut client) = connection.into_bytes_stream();

        client.write_all(&server_bytes)?;
        server.write_all(&client_bytes)?;

        blocking_proxy(&addr, client, server)
    }
}
