use bytes::BytesMut;
use std::io::prelude::*;
use std::{collections::HashMap, fmt};

pub struct Request {
    pub method: Method,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>,
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Request")
    }
}

pub struct Response {
    pub code: u16,
    pub content: Content,
    pub headers: HashMap<String, String>,
    pub content_encoding: Option<Encoding>,
}

impl Response {
    pub fn new(code: u16, content: Content) -> Self {
        Response {
            code,
            content,
            headers: HashMap::new(),
            content_encoding: None,
        }
    }
}

pub enum Content {
    Text(String),
    Bytes(Vec<u8>),
    OctetStream(Vec<u8>),
    Empty,
}

pub enum Encoding {
    Gzip,
}

impl From<Response> for Vec<u8> {
    fn from(mut response: Response) -> Self {
        let mut content_bytes = Vec::new();
        match response.content {
            Content::Text(text_content) => {
                response
                    .headers
                    .insert("Content-Type".into(), "text/plain".into());

                content_bytes = text_content.into_bytes();
            }
            Content::Bytes(bytes) => {
                content_bytes = bytes;
            }
            Content::OctetStream(bytes) => {
                response
                    .headers
                    .insert("Content-Type".into(), "application/octet-stream".into());
                content_bytes = bytes;
            }
            Content::Empty => {}
        };

        // encoding
        match response.content_encoding {
            None => {}
            Some(Encoding::Gzip) => {
                use flate2::{Compression, write::GzEncoder};
                response
                    .headers
                    .insert("Content-Encoding".into(), "gzip".into());

                // encode content bytes
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

                // TODO: handle the unwrap
                encoder.write_all(&content_bytes).unwrap();
                content_bytes = encoder.finish().unwrap();
            }
        }

        // calculate the content length
        response
            .headers
            .insert("Content-Length".into(), content_bytes.len().to_string());

        // Construct headers
        let code_and_reason = match response.code {
            200 => "200 OK",
            201 => "201 Created",
            404 => "404 Not Found",
            _ => "500 Internal Server Error",
        };

        let mut header_str = format!("HTTP/1.1 {}\r\n", code_and_reason);
        for (k, v) in response.headers {
            header_str.push_str(format!("{}: {}\r\n", k, v).as_str());
        }
        header_str.push_str("\r\n");

        let mut response_bytes = header_str.into_bytes();
        response_bytes.extend_from_slice(content_bytes.as_ref());

        response_bytes
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Method {
    Get,
    Post,
}

#[derive(thiserror::Error, Debug)]
pub enum DecodeHttpError {
    #[error("Invalid header.")]
    InvalidHeader,
    #[error("Invalid method.")]
    InvalidMethod(String),
    #[error("Error parsing content.")]
    InvalidContent,
}

pub fn decode_http_request(buf: BytesMut) -> Result<Request, DecodeHttpError> {
    // find the end of the headers
    let Some(headers_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
        return Err(DecodeHttpError::InvalidHeader);
    };

    // Extract headers as text
    let Ok(headers) = str::from_utf8(&buf[..headers_end]) else {
        return Err(DecodeHttpError::InvalidHeader);
    };

    let mut headers = headers.lines();
    let Some(request_line) = headers.next() else {
        return Err(DecodeHttpError::InvalidHeader);
    };

    let mut request_line = request_line.split_whitespace();
    let (Some(method), Some(request_uri), Some(_http_version)) = (
        request_line.next(),
        request_line.next(),
        request_line.next(),
    ) else {
        return Err(DecodeHttpError::InvalidHeader);
    };

    let method = match method {
        "GET" => Method::Get,
        "POST" => Method::Post,
        _ => return Err(DecodeHttpError::InvalidMethod(method.into())),
    };

    // parse headers
    let mut headers_map = HashMap::new();
    for header_line in headers {
        let header = header_line.split_once(":");
        let Some((header_name, header_value)) = header else {
            return Err(DecodeHttpError::InvalidHeader);
        };
        headers_map.insert(
            header_name.trim().to_string(),
            header_value.trim().to_string(),
        );
    }

    // parse content
    let mut content = Vec::new();
    let content_start = headers_end + 4;
    if let (Some(content_type), Some(content_length)) = (
        headers_map.get("Content-Type"),
        headers_map.get("Content-Length"),
    ) {
        let Ok(content_length_usize) = content_length.parse::<usize>() else {
            return Err(DecodeHttpError::InvalidContent);
        };

        if content_type == "application/octet-stream" {
            content.extend_from_slice(&buf[content_start..content_start + content_length_usize]);
        }
    }

    Ok(Request {
        method,
        uri: request_uri.into(),
        content,
        headers: headers_map,
    })
}
