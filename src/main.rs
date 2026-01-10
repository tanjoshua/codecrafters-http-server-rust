use bytes::BytesMut;
mod h1;
use h1::{decode_http_request, Method, Request, Response};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        let (stream, socket_addr) = listener.accept().await?;
        tokio::spawn(async move {
            println!("New connection: {}", socket_addr);
            if let Err(e) = process(stream).await {
                eprintln!("Could not process connection: {}", e);
            }
        });
    }
}

async fn process(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(4096);
    stream.read_buf(&mut buf).await?;

    let request = decode_http_request(buf);

    match request {
        Ok(request) => {
            println!("{:?} request received at {}", request.method, request.uri);
            let response = handle_request(request);
            let response_bytes: Vec<u8> = response.into();
            if stream.write_all(&response_bytes).await.is_err() {
                eprintln!("Error writing response");
            }
        }
        Err(e) => {
            eprintln!("Error occurred {}", e);
        }
    }

    Ok(())
}

fn handle_request(request: Request) -> Response {
    match (request.method, request.uri.as_str()) {
        (Method::Get, "/") => {
            println!("request sent to / endpoint");
            Response {
                code: 200,
                content: None,
            }
        }
        (_, _) => Response {
            code: 404,
            content: None,
        },
    }
}
