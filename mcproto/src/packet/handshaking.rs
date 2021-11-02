use super::{Packet, PacketBuilder, PacketRead, PacketWrite};
use crate::ReadExt;
use std::io::{Error, ErrorKind, Read, Result, Write};

// i hate it here
// https://wiki.vg/Minecraft_Forge_Handshake
#[derive(Debug)]
pub enum ForgeHandshake {
    // maybe remove this and wrap with option?
    None,

    // forge 1.7 - 1.12
    Version1,

    // forge 1.13+
    Version2,
}

impl ForgeHandshake {
    fn separate_address(address: String) -> (String, Self) {
        if !address.contains("\0") {
            (address, Self::None)
        } else {
            let (address, fml) = address.split_once("\0").unwrap();

            let forge = match fml {
                "FML\0" => Self::Version1,
                "FML2\0" => Self::Version2,

                // should definately warn about this somehow
                _ => Self::None,
            };

            (address.to_owned(), forge)
        }
    }

    fn net_id(&self) -> &str {
        match self {
            Self::None => "",
            Self::Version1 => "\0FML\0",
            Self::Version2 => "\0FML2\0",
        }
    }
}

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,

    pub forge: ForgeHandshake,
}

impl Handshake {
    fn modified_address(&self) -> String {
        format!(
            "{}{}",
            self.server_address,
            self.forge.net_id(),
        )
    }
}

impl Packet for Handshake {
    const PACKET_ID: i32 = 0;
}

impl PacketRead for Handshake {
    fn read<R: Read>(reader: &mut R) -> Result<Handshake> {
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

        let (server_address, forge) = ForgeHandshake::separate_address(server_address);

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            next_state,
            forge,
        })
    }
}

impl PacketWrite for Handshake {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut packet = PacketBuilder::new(Self::PACKET_ID)?;
        packet.write_varint(self.protocol_version)?;
        packet.write_string(self.modified_address())?;
        packet.write_ushort(self.server_port)?;
        packet.write_varint(self.next_state)?;
        Ok(packet.write(writer)?)
    }
}
