use crate::config::Config;
use crate::h1::{Content, Request, Response};
use std::sync::Arc;

// Code crafters specific endpoints
pub fn handle_user_agent(request: Request) -> Response {
    let Some(user_agent) = request.headers.get("User-Agent") else {
        return Response::new(400, Content::Text("No user agent found".into()));
    };
    Response::new(200, Content::Text(user_agent.clone()))
}

pub fn handle_post_files(config: Arc<Config>, request: Request) -> Response {
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

pub fn handle_files(config: Arc<Config>, request: Request) -> Response {
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

pub fn handle_echo(request: Request) -> Response {
    let mut tokens = request.uri.split("/echo/");
    match tokens.nth(1) {
        Some(message) => Response::new(200, Content::Text(message.to_string())),
        None => Response::new(400, Content::Text(String::from("No string found"))),
    }
}
