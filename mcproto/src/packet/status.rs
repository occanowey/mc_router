use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Error, ErrorKind, Read, Result, Write};
use packet_derive::Packet;

#[derive(Debug, Packet)]
#[id(0)]
pub struct Request;

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

impl PacketWrite for Request {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let packet = PacketBuilder::new(Self::PACKET_ID)?;
        Ok(packet.write(writer)?)
    }
}

#[derive(Debug, Packet)]
#[id(0)]
pub struct Response { pub response: String }

impl PacketRead for Response {
    fn read<R: Read>(reader: &mut R) -> Result<Response> {
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

        let (response, _) = reader.read_string()?;

        Ok(Response { response })
    }
}

impl PacketWrite for Response {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_string(&self.response)?;
        Ok(packet.write(writer)?)
    }
}

#[derive(Debug, Packet)]
#[id(1)]
pub struct Ping { pub data: i64 }

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

impl PacketWrite for Ping {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_long(self.data)?;
        Ok(packet.write(writer)?)
    }
}

#[derive(Debug, Packet)]
#[id(1)]
pub struct Pong { pub data: i64 }

impl PacketRead for Pong {
    fn read<R: Read>(reader: &mut R) -> Result<Pong> {
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

        let data = reader.read_long()?;

        Ok(Pong { data })
    }
}

impl PacketWrite for Pong {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_long(self.data)?;
        Ok(packet.write(writer)?)
    }
}
