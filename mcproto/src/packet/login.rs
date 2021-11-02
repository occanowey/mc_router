use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Error, ErrorKind, Read, Result, Write};

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
}

impl Packet for LoginStart {
    const PACKET_ID: i32 = 0;
}

impl PacketRead for LoginStart {
    fn read<R: Read>(reader: &mut R) -> Result<LoginStart> {
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

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
