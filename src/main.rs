mod cc;
mod config;
mod h1;
use config::Config;
use h1::{Content, Method, Request, Response, decode_http_request};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use bytes::{Buf, BytesMut};

use crate::h1::Encoding;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let dir = args
        .iter()
        .position(|arg| arg == "--directory")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let config = Arc::new(Config { directory: dir });

    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        let (stream, socket_addr) = listener.accept().await?;
        let config = Arc::clone(&config);
        tokio::spawn(async move {
            println!("New connection: {}", socket_addr);
            if let Err(e) = process(config, stream).await {
                eprintln!("Could not process connection: {}", e);
            }
        });
    }
}
async fn process(
    config: Arc<Config>,
    mut stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(4096);

    loop {
        let n = stream.read_buf(&mut buf).await?;
        if n == 0 {
            break;
        }

        let request = decode_http_request(&mut buf);

        match request {
            Ok((request, bytes_read)) => {
                println!("{:?} request received at {}", request.method, request.uri);

                // advance buffer
                buf.advance(bytes_read);

                // get encodings for response before request passed to handler
                let encodings = request.headers.get("Accept-Encoding").cloned();

                // check whether tcp connection should be persisted
                let should_close = request
                    .headers
                    .get("Connection")
                    .is_some_and(|v| v == "close");

                let mut response = handle_request(config.clone(), request);

                // set close header
                if should_close {
                    response.headers.insert("Connection".into(), "close".into());
                }

                // set encoding,
                if let Some(encodings) = encodings
                    && encodings.split(",").map(|s| s.trim()).any(|s| s == "gzip")
                {
                    response.content_encoding = Some(Encoding::Gzip);
                }

                let response_bytes: Vec<u8> = response.into();
                if stream.write_all(&response_bytes).await.is_err() {
                    eprintln!("Error writing response");
                }

                // close connection if header was set
                if should_close {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error occurred {}", e);
            }
        }
    }

    Ok(())
}

fn handle_request(config: Arc<Config>, request: Request) -> Response {
    if request.uri.starts_with("/echo/") && request.method == Method::Get {
        return cc::handle_echo(request);
    }

    if request.uri.starts_with("/files/") && request.method == Method::Get {
        return cc::handle_files(config, request);
    }

    if request.uri.starts_with("/files/") && request.method == Method::Post {
        return cc::handle_post_files(config, request);
    }

    if request.uri == "/user-agent" && request.method == Method::Get {
        return cc::handle_user_agent(request);
    }

    match (request.method, request.uri.as_str()) {
        (Method::Get, "/") => Response::new(200, Content::Empty),
        (_, _) => Response::new(404, Content::Empty),
    }
}
