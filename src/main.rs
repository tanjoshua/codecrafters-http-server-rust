use std::sync::Arc;

use bytes::BytesMut;
mod h1;
use h1::{Content, Method, Request, Response, decode_http_request};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct Config {
    directory: Option<String>,
}

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
    stream.read_buf(&mut buf).await?;

    let request = decode_http_request(buf);

    match request {
        Ok(request) => {
            println!("{:?} request received at {}", request.method, request.uri);
            // set encoding,
            let encoding = request.headers.get("Accept-Encoding").cloned();

            let mut response = handle_request(config, request);

            if let Some(encoding) = encoding
                && encoding == "gzip"
            {
                response
                    .headers
                    .insert("Content-Encoding".into(), "gzip".into());
            }

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

fn handle_request(config: Arc<Config>, request: Request) -> Response {
    if request.uri.starts_with("/echo/") && request.method == Method::Get {
        return handle_echo(request);
    }

    if request.uri.starts_with("/files/") && request.method == Method::Get {
        return handle_files(config, request);
    }

    if request.uri.starts_with("/files/") && request.method == Method::Post {
        return handle_post_files(config, request);
    }

    match (request.method, request.uri.as_str()) {
        (Method::Get, "/") => Response::new(200, Content::Empty),
        (Method::Get, "/user-agent") => {
            let Some(user_agent) = request.headers.get("User-Agent") else {
                return Response::new(400, Content::Text("No user agent found".into()));
            };
            Response::new(200, Content::Text(user_agent.clone()))
        }
        (_, _) => Response::new(404, Content::Empty),
    }
}

fn handle_post_files(config: Arc<Config>, request: Request) -> Response {
    let mut tokens = request.uri.split("/files/");
    let Some(filename) = tokens.nth(1) else {
        return Response::new(400, Content::Text("No filename found".into()));
    };

    let Some(dir) = &config.directory else {
        return Response::new(404, Content::Text("No file directory found".into()));
    };

    let Ok(_) = std::fs::write(format!("{}{}", dir, filename), request.content) else {
        return Response::new(404, Content::Text("Directory path does not exist".into()));
    };

    Response::new(201, Content::Empty)
}

fn handle_files(config: Arc<Config>, request: Request) -> Response {
    let mut tokens = request.uri.split("/files/");
    let Some(filename) = tokens.nth(1) else {
        return Response::new(400, Content::Text("No filename found".into()));
    };

    match &config.directory {
        None => Response::new(404, Content::Text("No file directory found".into())),
        Some(dir) => {
            let Ok(file) = std::fs::read(format!("{}{}", dir, filename)) else {
                return Response::new(404, Content::Text("File not found".into()));
            };

            Response::new(200, Content::OctetStream(file))
        }
    }
}

fn handle_echo(request: Request) -> Response {
    let mut tokens = request.uri.split("/echo/");
    match tokens.nth(1) {
        Some(message) => Response::new(200, Content::Text(message.to_string())),
        None => Response::new(400, Content::Text(String::from("No string found"))),
    }
}
