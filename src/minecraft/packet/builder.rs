use crate::minecraft::WriteExt;
use std::io::{Result, Write};

macro_rules! builder_write_type {
    ($name:tt, $type:ty) => {
        pub fn $name(&mut self, value: $type) -> Result<()> {
            self.buffer.$name(value)
        }
    };
}

#[derive(Debug)]
pub struct PacketBuilder {
    id: i32,
    buffer: Vec<u8>,
}

impl PacketBuilder {
    pub fn new(id: i32) -> Result<PacketBuilder> {
        let mut buffer = Vec::new();
        buffer.write_varint(id)?;

        Ok(PacketBuilder { id, buffer })
    }

    builder_write_type!(write_boolean, bool);
    builder_write_type!(write_byte, i8);
    builder_write_type!(write_ubyte, u8);
    builder_write_type!(write_short, i16);
    builder_write_type!(write_ushort, u16);
    builder_write_type!(write_int, i32);
    builder_write_type!(write_long, i64);
    builder_write_type!(write_float, f32);
    builder_write_type!(write_double, f64);
    builder_write_type!(write_varint, i32);

    pub fn write_string<S: Into<String>>(&mut self, value: S) -> Result<()> {
        self.buffer.write_string(value)
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_varint(self.buffer.len() as i32)?;
        writer.write_all(&self.buffer)
    }
}
