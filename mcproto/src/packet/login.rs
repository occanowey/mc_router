use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Read, Result};
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
    fn write_data(&self, packet: &mut PacketBuilder) -> Result<()> {
        packet.write_string(&self.username)?;

        Ok(())
    }
}
