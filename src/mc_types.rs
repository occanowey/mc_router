use std::io::{Read, Result, Write};

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

macro_rules! write_named_primitive {
    ($name:tt, $length:expr, $primitive:ty) => {
        #[inline]
        fn $name(&mut self, value: $primitive) -> Result<()> {
            self.write_all(&value.to_be_bytes())
        }
    };
}

impl<R: Read + ?Sized> ReadMCTypesExt for R {}
impl<W: Write + ?Sized> WriteMCTypesExt for W {}

/// Extends [`Read`] with methods for reading various Minecraft protocol data types.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub trait ReadMCTypesExt: Read {
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

/// Extends [`Write`] with methods for writing various Minecraft protocol data types.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
pub trait WriteMCTypesExt: Write {
    // Boolean
    #[inline]
    fn write_boolean(&mut self, value: bool) -> Result<()> {
        self.write_ubyte(value as u8)
    }

    // Byte
    write_named_primitive!(write_byte, 1, i8);

    // Unsigned Byte
    write_named_primitive!(write_ubyte, 1, u8);

    // Short
    write_named_primitive!(write_short, 2, i16);

    // Unsigned Short
    write_named_primitive!(write_ushort, 2, u16);

    // Int
    write_named_primitive!(write_int, 4, i32);

    // Long
    write_named_primitive!(write_long, 8, i64);

    // Float
    write_named_primitive!(write_float, 4, f32);

    // Double
    write_named_primitive!(write_double, 8, f64);

    // String
    #[inline]
    fn write_string<S: Into<String>>(&mut self, value: S) -> Result<()> {
        let value = value.into();
        self.write_varint(value.len() as i32)?;
        self.write_all(value.as_bytes())
    }

    // Chat

    // Identifier

    // VarInt
    #[inline]
    fn write_varint(&mut self, value: i32) -> Result<()> {
        let mut input = value as u32;

        loop {
            if (input & 0xFFFFFF80) == 0 {
                break;
            }

            self.write_ubyte((input & 0x7F | 0x80) as u8)?;
            input >>= 7;
        }

        self.write_ubyte((input & 0xFF) as u8)
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
