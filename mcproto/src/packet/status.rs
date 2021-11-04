use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Read, Result};
use packet_derive::Packet;

#[derive(Debug, Packet)]
#[id(0)]
pub struct Request;

impl PacketRead for Request {
    fn read_data<R: Read>(_: &mut R) -> Result<Request> {
        Ok(Request)
    }
}

impl PacketWrite for Request {
    fn write_data(&self, _: &mut PacketBuilder) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Packet)]
#[id(0)]
pub struct Response { pub response: String }

impl PacketRead for Response {
    fn read_data<R: Read>(reader: &mut R) -> Result<Response> {
        let (response, _) = reader.read_string()?;

        Ok(Response { response })
    }
}

impl PacketWrite for Response {
    fn write_data(&self, packet: &mut PacketBuilder) -> Result<()> {
        packet.write_string(&self.response)?;

        Ok(())
    }
}

#[derive(Debug, Packet)]
#[id(1)]
pub struct Ping { pub data: i64 }

impl PacketRead for Ping {
    fn read_data<R: Read>(reader: &mut R) -> Result<Ping> {
        let data = reader.read_long()?;

        Ok(Ping { data })
    }
}

impl PacketWrite for Ping {
    fn write_data(&self, packet: &mut PacketBuilder) -> Result<()> {
        packet.write_long(self.data)?;

        Ok(())
    }
}

#[derive(Debug, Packet)]
#[id(1)]
pub struct Pong { pub data: i64 }

impl PacketRead for Pong {
    fn read_data<R: Read>(reader: &mut R) -> Result<Pong> {
        let data = reader.read_long()?;

        Ok(Pong { data })
    }
}

impl PacketWrite for Pong {
    fn write_data(&self, packet: &mut PacketBuilder) -> Result<()> {
        packet.write_long(self.data)?;

        Ok(())
    }
}
