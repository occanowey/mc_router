use std::io::{Read, Result};

macro_rules! read_named_primitive {
    ($name:tt, $length:expr, $primitive:ty) => {
        #[inline]
        fn $name(&mut self) -> Result<$primitive> {
            let mut buffer = [0; $length];
            self.read_exact(&mut buffer)?;
            Ok(<$primitive>::from_be_bytes(buffer))
        }
    };
}

impl<R: Read + ?Sized> MinecraftReadExt for R {}

/// Extends [`Read`] with methods for reading various Minecraft protocol data types.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub trait MinecraftReadExt: Read {
    // Boolean
    #[inline]
    fn read_boolean(&mut self) -> Result<bool> {
        Ok(self.read_ubyte()? & 1 == 1)
    }

    // Byte
    read_named_primitive!(read_byte, 1, i8);

    // Unsigned Byte
    read_named_primitive!(read_ubyte, 1, u8);

    // Short
    read_named_primitive!(read_short, 2, i16);

    // Unsigned Short
    read_named_primitive!(read_ushort, 2, u16);

    // Int
    read_named_primitive!(read_int, 4, i32);

    // Long
    read_named_primitive!(read_long, 8, i64);

    // Float
    read_named_primitive!(read_float, 4, f32);

    // Double
    read_named_primitive!(read_double, 8, f64);

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
