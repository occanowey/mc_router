use std::io::{Read, Result};

/// Extends [`Read`] with methods for reading various Minecraft protocol data types.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub trait ReadMCTypesExt: Read {
    // Boolean

    // Byte

    // Unsigned Byte
    #[inline]
    fn read_ubyte(&mut self) -> Result<u8> {
        let mut buffer = [0; 1];
        self.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    // Short

    // Unsigned Short
    #[inline]
    fn read_ushort(&mut self) -> Result<u16> {
        let mut buffer = [0; 2];
        self.read_exact(&mut buffer)?;
        Ok(u16::from_be_bytes(buffer))
    }

    // Int

    // Long

    // Float

    // Double

    // String
    #[inline]
    fn read_string(&mut self) -> Result<(String, usize)> {
        let (string_len, len_len) = self.read_varint()?;
        let mut buffer = vec![0; string_len as usize];
        self.read_exact(&mut buffer)?;
        let string = String::from_utf8(buffer).unwrap();

        Ok((string, string_len as usize + len_len))
    }

    // Chat

    // Identifier

    // VarInt
    #[inline]
    fn read_varint(&mut self) -> Result<(i32, usize)> {
        let mut acc = 0;
        let mut i = 0;

        loop {
            let byte = self.read_ubyte()? as i32;
            acc |= (byte & 0x7F) << (i * 7);

            i += 1;
            if i > 5 {
                panic!("varint too big");
            }

            if (byte & 0b10000000) == 0 {
                break;
            }
        }

        Ok((acc, i))
    }

    // VarLong

    // Entity Metadata

    // Slot

    // NBT Tag

    // Position

    // Angle

    // UUID

    // Optional X

    // Array of X

    // X Enum

    // Byte Array
}

impl<R: Read + ?Sized> ReadMCTypesExt for R {}
