use bytes::BytesMut;
use std::fmt;

pub struct Request {
    pub method: Method,
    pub uri: String,
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Request")
    }
}

pub struct Response {
    pub code: u16,
    pub content: Content,
}

pub enum Content {
    Text(String),
    Bytes(Vec<u8>),
    Empty,
}

impl From<Response> for Vec<u8> {
    fn from(response: Response) -> Self {
        let code_and_reason = match response.code {
            200 => "200 OK",
            404 => "404 Not Found",
            _ => "500 Internal Server Error",
        };

        let mut header_str = format!("HTTP/1.1 {}\r\n", code_and_reason);
        let mut content_bytes = Vec::new();
        match response.content {
            Content::Text(text_content) => {
                let headers = format!(
                    "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                    text_content.len()
                );
                header_str.push_str(headers.as_str());
                content_bytes = text_content.into_bytes();
            }
            Content::Bytes(bytes) => {
                let headers = format!("Content-Length: {}\r\n\r\n", bytes.len());
                header_str.push_str(headers.as_str());
                content_bytes = bytes;
            }
            Content::Empty => header_str.push_str("\r\n"),
        };

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

    Ok(Request {
        method,
        uri: request_uri.into(),
    })
}
