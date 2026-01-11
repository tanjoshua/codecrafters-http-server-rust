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
            let response = handle_request(config, request);
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

    match (request.method, request.uri.as_str()) {
        (Method::Get, "/") => Response {
            code: 200,
            content: Content::Empty,
        },
        (Method::Get, "/user-agent") => {
            let Some(user_agent) = request.headers.get("User-Agent") else {
                return Response {
                    code: 400,
                    content: Content::Text("No user agent found".into()),
                };
            };
            Response {
                content: Content::Text(user_agent.clone()),
                code: 200,
            }
        }
        (_, _) => Response {
            code: 404,
            content: Content::Empty,
        },
    }
}

fn handle_files(config: Arc<Config>, request: Request) -> Response {
    let mut tokens = request.uri.split("/files/");
    let Some(filename) = tokens.nth(1) else {
        return Response {
            code: 400,
            content: Content::Text("No filename found".into()),
        };
    };

    match &config.directory {
        None => Response {
            code: 404,
            content: Content::Text("No file directory found".into()),
        },
        Some(dir) => {
            let Ok(file) = std::fs::read(format!("{}{}", dir, filename)) else {
                return Response {
                    code: 404,
                    content: Content::Text("File not found".into()),
                };
            };

            Response {
                code: 200,
                content: Content::OctetStream(file),
            }
        }
    }
}

fn handle_echo(request: Request) -> Response {
    let mut tokens = request.uri.split("/echo/");
    match tokens.nth(1) {
        Some(message) => Response {
            code: 200,
            content: Content::Text(message.to_string()),
        },
        None => Response {
            code: 400,
            content: Content::Text(String::from("No string found")),
        },
    }
}
