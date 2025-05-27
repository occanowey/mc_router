// TODO: this should probably be merged into mcproto at some point

use std::{
    convert::TryInto,
    io::{Read as _, Write},
    net::TcpStream,
};

use bytes::{Buf, BufMut, BytesMut};
use color_eyre::Result;
use mcproto::{
    packet_derive::BufType,
    types::{BufType, ReadError},
};
use tracing::{trace, warn};

pub fn maybe_handle_legacy_status(stream: &mut TcpStream) -> Result<bool> {
    let mut buf = vec![0; 3];
    let len = stream.peek(&mut buf)?;
    assert!(len >= 1, "todo: test how peek works with eof");

    if len == 1 && &buf[0..1] == b"\xfe" {
        let motd = "A Minecraft Server";
        let online_players = 0;
        let max_players = 13;

        let status_line = format!("{}§{}§{}", motd, online_players, max_players);

        write_legacy_kick_packet(stream, status_line)?;

        return Ok(true);
    }

    if len == 2 && &buf[0..2] == b"\xfe\x01" {
        let protocol_version = 47;
        let minecraft_version = "1.4.2";
        let motd = "A Minecraft Server";
        let online_players = 0;
        let max_players = 14;

        let status_line = format!(
            "§1\0{}\0{}\0{}\0{}\0{}",
            protocol_version, minecraft_version, motd, online_players, max_players
        );

        write_legacy_kick_packet(stream, status_line)?;
        return Ok(true);
    }

    if len >= 3 && &buf[0..3] == b"\xfe\x01\xfa" {
        // skip peeked bytes
        stream.read_exact(&mut buf)?;

        let mut buffer = BytesMut::new();

        let ping_request = loop {
            let mut read_buffer = [0; 64];
            let len = stream.read(&mut read_buffer)?;
            buffer.put(&read_buffer[..len]);

            match Legacy16PingRequest::buf_read_len(&mut buffer.clone()) {
                Ok((request, request_len)) => {
                    if request.plugin_message_id != "MC|PingHost" {
                        warn!(expected = "MC|PingHost", recieved = ?request.plugin_message_id, "unexpected ping plugin message id");
                    }

                    let expected_len = 2                                + // length of id length
                        (request.plugin_message_id.encode_utf16().count() * 2) + // length of id
                        2                                                      + // length of length
                        request.plugin_message_length as usize;

                    if request_len != expected_len {
                        warn!(
                            expected = expected_len,
                            recieved = request_len,
                            "unexpected ping plugin message id"
                        );
                    }

                    break request;
                }
                Err(ReadError::ReadOutOfBounds(..)) => {}
                Err(other) => return Err(other.into()),
            }
        };
        trace!(?ping_request, "legacy ping request");

        let protocol_version = 73;
        let minecraft_version = "1.6.1";
        let motd = "A Minecraft Server";
        let online_players = 0;
        let max_players = 16;

        let status_line = format!(
            "§1\0{}\0{}\0{}\0{}\0{}",
            protocol_version, minecraft_version, motd, online_players, max_players
        );

        write_legacy_kick_packet(stream, status_line)?;

        return Ok(true);
    }

    Ok(false)
}

#[derive(Debug, BufType)]
struct Legacy16PingRequest {
    // mildly funny structure bc i'm not reading proper packets
    #[buftype(read_with = "utf16_be_string::buf_read_len")]
    plugin_message_id: String,
    plugin_message_length: u16,

    protocol_version: u8,
    #[buftype(read_with = "utf16_be_string::buf_read_len")]
    hostname: String,
    port: i32,
}

pub mod utf16_be_string {
    use super::*;

    // not curently used
    // pub fn buf_read<B: Buf>(buf: &mut B) -> Result<String, ReadError> {
    //     self::buf_read_len(buf).map(|value| value.0)
    // }

    pub fn buf_read_len<B: Buf>(buf: &mut B) -> Result<(String, usize), ReadError> {
        let (string_len, string_len_len) = u16::buf_read_len(buf)?;
        let string_len = string_len as usize * 2;

        if buf.remaining() < string_len {
            return Err(ReadError::ReadOutOfBounds(buf.remaining(), string_len));
        }

        let string_buf = buf.copy_to_bytes(string_len);
        let string_buf =
            string_buf
                .chunks(2)
                .map(|window| {
                    u16::from_be_bytes(window.try_into().expect(
                        "window size was changed from 2 or string_len wasn't multiplied by 2",
                    ))
                })
                .collect::<Vec<_>>();

        let string = String::from_utf16_lossy(&string_buf);
        Ok((string, string_len_len + string_len))
    }

    // not currently used
    // pub fn buf_write<B: BufMut>(_value: &str, _buf: &mut B) {
    //     todo!("utf16_string::buf_write")
    // }
}

fn write_legacy_kick_packet<S: AsRef<str>>(stream: &mut TcpStream, status_line: S) -> Result<()> {
    let status_line = status_line.as_ref();

    // note this is code units (`.encode_utf16()`) rather than characters (`.chars()`)
    // todo: check it actually fits in a u16
    let status_line_len = status_line.encode_utf16().count();

    let mut status_buf = Vec::with_capacity(
        1 + // packet id length
        2 + // status line length length
        status_line_len * 2, // length of bytes in utf16 encoded status line
    );

    status_buf.push(0xff); // kick packet id
    status_buf.extend((status_line_len as u16).to_be_bytes()); // status line length
    status_buf.extend(status_line.encode_utf16().flat_map(|c| c.to_be_bytes())); // status line

    stream.write_all(&status_buf)?;

    Ok(())
}
