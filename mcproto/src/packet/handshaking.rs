use super::PacketBuilder;
use crate::ReadExt;
use std::io::{Error, ErrorKind, Read, Result, Write};

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,

    pub fml: bool,
}

impl Handshake {
    pub const PACKET_ID: i32 = 0;

    pub fn read<R: Read>(reader: &mut R) -> Result<Handshake> {
        // todo: maybe handle legacy ping?
        let _length = reader.read_varint()?;

        let (id, _) = reader.read_varint()?;
        if id != Self::PACKET_ID {
            return Err(Error::new(ErrorKind::Other, "Invalid packet id"));
        }

        let (protocol_version, _) = reader.read_varint()?;
        let (server_address, _) = reader.read_string()?;
        let server_port = reader.read_ushort()?;
        let (next_state, _) = reader.read_varint()?;

        let fml = server_address.ends_with("\0FML\0");
        let server_address = server_address.replace("\0FML\0", "");

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            next_state,
            fml,
        })
    }

    fn get_fml_address(&self) -> String {
        format!(
            "{}{}",
            self.server_address,
            if self.fml { "\0FML\0" } else { "" }
        )
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_varint(self.protocol_version)?;
        packet.write_string(self.get_fml_address())?;
        packet.write_ushort(self.server_port)?;
        packet.write_varint(self.next_state)?;
        Ok(packet.write(writer)?)
    }
}
