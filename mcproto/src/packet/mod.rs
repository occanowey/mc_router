mod builder;

mod handshaking;
mod status;
mod login;

use std::io::{Read, Write, Result, Error, ErrorKind};

use crate::ReadExt;

pub use builder::PacketBuilder;

pub use handshaking::{ForgeHandshake, Handshake};
pub use status::{Request, Response, Ping, Pong};
pub use login::LoginStart;

pub trait Packet {
    const PACKET_ID: i32;
}

pub trait PacketRead: Packet + Sized {
    fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

        Self::read_data(reader)
    }

    /// Read fields after length & id
    fn read_data<R: Read>(reader: &mut R) -> Result<Self>;
}

pub trait PacketWrite: Packet {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()>;
}