mod builder;

mod handshaking;
mod status;
mod login;

use std::io::{Read, Write, Result};

pub use builder::PacketBuilder;

pub use handshaking::Handshake;
pub use status::{Request, Response, Ping, Pong};
pub use login::LoginStart;

pub trait Packet {
    const PACKET_ID: i32;
}

pub trait PacketRead: Packet + Sized {
    fn read<R: Read>(reader: &mut R) -> Result<Self>;
}

pub trait PacketWrite: Packet {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()>;
}