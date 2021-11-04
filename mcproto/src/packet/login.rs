use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Read, Result, Write};
use packet_derive::Packet;

#[derive(Debug, Packet)]
#[id(0)]
pub struct LoginStart {
    pub username: String,
}

impl PacketRead for LoginStart {
    fn read_data<R: Read>(reader: &mut R) -> Result<LoginStart> {
        let (username, _) = reader.read_string()?;

        Ok(LoginStart {
            username,
        })
    }
}

impl PacketWrite for LoginStart {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_string(&self.username)?;
        Ok(packet.write(writer)?)
    }
}
