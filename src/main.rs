use std::{io::{Write, Read}, net::TcpListener};

struct MCPacketReader<R: Read> {
    inner: R,
    buffer: Vec<u8>,
}

impl <R: Read> MCPacketReader<R> {
    fn new(inner: R) -> MCPacketReader<R> {
        MCPacketReader {
            inner: inner,
            buffer: Vec::new(),
        }
    }

    fn next_bytes(&mut self, length: usize) -> Vec<u8> {
        if self.buffer.len() < length {
            let mut chunk = [0; 16];
            let length = self.inner.read(&mut chunk).unwrap();
            self.buffer.write(&chunk[0..length]).unwrap();
        }

        let remaining = self.buffer.split_off(length);
        std::mem::replace(&mut self.buffer, remaining)
    }

    fn next_byte(&mut self) -> u8 {
        self.next_bytes(1)[0]
    }

    fn read_varint(&mut self) -> (i32, usize) {
        let mut acc = 0;
        let mut i = 0;

        loop {
            let byte = self.next_byte() as i32;
            acc |= ( byte & 0x7F ) << i * 7;
            
            i += 1;
            if i > 5 {
                panic!( "varint too big" );
            }

            if ( byte & 0b10000000 ) == 0 {
                break;
            }
        }

        (acc, i)
    }

    fn next_packet(&mut self) -> PacketBuffer {
        let (length, _) = self.read_varint();

        let (id, id_length) = self.read_varint();
        let data = self.next_bytes((length as usize) - id_length);

        PacketBuffer {id, data}
    }
}

struct PacketBuffer {
    id: i32,
    data: Vec<u8>
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8081").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("new connection");

        let mut reader = MCPacketReader::new(stream);
        
        let handshake = reader.next_packet();
        println!("handshake (id: {}, data: {:?})", handshake.id, handshake.data);
    }
}
