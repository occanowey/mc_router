use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8081").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        println!("new connection");
    }
}
