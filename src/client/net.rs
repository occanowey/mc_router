// This whole thing probably belongs in mcproto but I want to flesh it out first

use std::net::TcpStream;

use mcproto::packet::{Handshake, LoginStart};
use super::error::ClientError;
use self::sealed::*;

mod sealed {
    use mcproto::packet::PacketRead;

    // Not really sure this is strictly network related,
    // but it's all I'm using for so it stays here.
    pub trait NetworkState {}

    pub trait StateReadPacket<NetworkState>: PacketRead {}
}

// would rather this be in network handler but generics makes that difficult if not impossible
pub fn handler_from_stream(stream: TcpStream) -> NetworkHandler<Handshaking> {
    // `unwrap` may cause issues but I don't think it's very likely to get here without a valid connection
    let address = stream.peer_addr().unwrap().to_string();

    NetworkHandler {
        stream,
        address,

        state: Handshaking,
    }
}

pub struct NetworkHandler<S: NetworkState> {
    pub stream: TcpStream,
    pub address: String,

    #[allow(dead_code)]
    // future use... ?
    state: S,
}

impl<S: NetworkState> NetworkHandler<S> {
    pub fn read<P: StateReadPacket<S>>(&mut self) -> Result<P, ClientError> {
        Ok(P::read(&mut self.stream)?)
    }
}

//
// Handshaking State
//
pub struct Handshaking;
impl NetworkState for Handshaking {}

impl StateReadPacket<Handshaking> for Handshake {}

impl NetworkHandler<Handshaking> {
    pub fn status(self) -> NetworkHandler<Status> {
        NetworkHandler {
            stream: self.stream,
            address: self.address,
            state: Status,
        }
    }

    pub fn login(self) -> NetworkHandler<Login> {
        NetworkHandler {
            stream: self.stream,
            address: self.address,
            state: Login,
        }
    }
}

//
// Status State
//
pub struct Status;
impl NetworkState for Status {}

//
// Login State
//
pub struct Login;
impl NetworkState for Login {}

impl StateReadPacket<Login> for LoginStart {}

// play
