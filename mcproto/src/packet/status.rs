use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Error, ErrorKind, Read, Result, Write};

#[derive(Debug)]
pub struct Request;

impl Packet for Request {
    const PACKET_ID: i32 = 0;
}

impl PacketRead for Request {
    fn read<R: Read>(reader: &mut R) -> Result<Request> {
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

        Ok(Request)
    }
}

#[derive(Debug)]
pub struct Response { pub response: String }

impl Packet for Response {
    const PACKET_ID: i32 = 0;
}

impl PacketWrite for Response {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_string(&self.response)?;
        Ok(packet.write(writer)?)
    }
}

#[derive(Debug)]
pub struct Ping { pub data: i64 }

impl Packet for Ping {
    const PACKET_ID: i32 = 1;
}

impl PacketRead for Ping {
    fn read<R: Read>(reader: &mut R) -> Result<Ping> {
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

        let data = reader.read_long()?;

        Ok(Ping { data })
    }
}

#[derive(Debug)]
pub struct Pong { pub data: i64 }

impl Packet for Pong {
    const PACKET_ID: i32 = 1;
}

impl PacketWrite for Pong {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_long(self.data)?;
        Ok(packet.write(writer)?)
    }
}
