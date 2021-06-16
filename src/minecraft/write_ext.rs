use std::io::{Result, Write};

macro_rules! write_named_primitive {
    ($name:tt, $length:expr, $primitive:ty) => {
        #[inline]
        fn $name(&mut self, value: $primitive) -> Result<()> {
            self.write_all(&value.to_be_bytes())
        }
    };
}

impl<W: Write + ?Sized> MinecraftWriteExt for W {}

/// Extends [`Write`] with methods for writing various Minecraft protocol data types.
///
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
pub trait MinecraftWriteExt: Write {
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
