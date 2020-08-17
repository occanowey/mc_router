use std::{
    io::{BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
};

fn read_varint(offset: usize, src: &[u8]) -> (i32, usize) {
    let mut acc = 0;
    let mut i = 0;

    loop {
        let byte = src[offset + i] as i32;
        acc |= (byte & 0x7F) << (i * 7);

        i += 1;
        if i > 5 {
            panic!("varint too big");
        }

        if (byte & 0b10000000) == 0 {
            break;
        }
    }

    (acc, i)
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8081").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("new connection");

        handle_client(stream);
    }
}

fn handle_client(client: TcpStream) {
    let mut client = BufReader::new(client);

    let mut index = 0;
    // 7 because it's the minimum length of a handshake packet but more than a varint :)
    let mut buffer = vec![0u8; 7];
    client.read_exact(&mut buffer).unwrap();

    // todo: handle legacy ping

    let (length, length_length) = read_varint(index, &buffer);
    index += length_length;

    // length of the packet - what's already in the buffer
    let chunk_length = (length as usize) - (buffer.len() - length_length);
    let mut chunk = vec![0; chunk_length];
    client.read_exact(&mut chunk).unwrap();
    buffer.append(&mut chunk);

    let (id, id_length) = read_varint(index, &buffer);
    index += id_length;

    if id != 0 {
        panic!("Invalid packet id recieved from client.");
    }

    let (protocol_version, protocol_version_length) = read_varint(index, &buffer);
    index += protocol_version_length;

    // messy
    let server_address = {
        let (length, length_length) = read_varint(index, &buffer);
        index += length_length;
        let data = &buffer[index..index + length as usize];
        index += length as usize;
        String::from_utf8(data.to_vec()).unwrap()
    };

    let server_port = ((buffer[index] as u16) << 8) | buffer[index + 1] as u16;
    index += 2;

    let (next_state, _) = read_varint(index, &buffer);

    println!("handshake (id: {}) {{", id);
    println!("\tprotocol version = {}", protocol_version);
    println!("\tserver address = {}", server_address);
    println!("\tserver port = {}", server_port);
    println!("\tnext state = {}", next_state);
    println!("}}");

    let mut server = TcpStream::connect("127.0.0.1:25565").unwrap();
    server.write_all(&buffer).unwrap();
    server.write_all(client.buffer()).unwrap();

    let mut client_read = client.into_inner();
    let mut client_write = client_read.try_clone().unwrap();

    let mut server_read = server;
    let mut server_write = server_read.try_clone().unwrap();

    let c2s_connected = Arc::new(RwLock::new(true));
    let s2c_connected = c2s_connected.clone();

    // c -> s
    thread::spawn(move || {
        let mut buffer = vec![0; 128];
        while *c2s_connected.read().unwrap() {
            let length = client_read.read(&mut buffer).unwrap();

            if length > 0 {
                server_write.write(buffer.get(0..length).unwrap()).unwrap();
            } else {
                *c2s_connected.write().unwrap() = false;
            }
        }
    });

    // s -> c
    thread::spawn(move || {
        let mut buffer = vec![0; 128];
        while *s2c_connected.read().unwrap() {
            let length = server_read.read(&mut buffer).unwrap();

            if length > 0 {
                client_write.write(buffer.get(0..length).unwrap()).unwrap();
            } else {
                *s2c_connected.write().unwrap() = false;
            }
        }
    });
}
