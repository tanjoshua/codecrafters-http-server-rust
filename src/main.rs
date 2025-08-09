use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    println!("accepted new connection");
    let reader = BufReader::new(&stream);
    let request = reader.lines().next().unwrap().unwrap();
    match &request[..] {
        "GET / HTTP/1.1" => {
            let resp = "HTTP/1.1 200 OK\r\n\r\n";
            stream.write_all(resp.as_bytes()).unwrap();
        }
        _ => {
            let resp = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
            stream.write_all(resp.as_bytes()).unwrap();
        }
    }
}
