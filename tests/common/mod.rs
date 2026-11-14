//! A one-shot HTTP stub on a std `TcpListener`: serves canned responses in order and hands back the
//! request heads it saw (lowercased).

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

pub struct Stub {
    pub base_url: String,
    requests: Receiver<String>,
}

impl Stub {
    /// The next request head (request line and headers), lowercased.
    pub fn request(&self) -> String {
        self.requests
            .recv_timeout(Duration::from_secs(5))
            .expect("the stub got no request")
    }
}

pub fn serve(responses: Vec<String>) -> Stub {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, requests) = mpsc::channel();
    thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
                head.push_str(&line);
            }
            stream.write_all(response.as_bytes()).unwrap();
            let _ = tx.send(head.to_lowercase());
        }
    });
    Stub { base_url, requests }
}

pub fn response(status: &str, content_type: &str, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n{headers}\r\n{body}",
        body.len()
    )
}

pub fn json(body: &str) -> String {
    response("200 OK", "application/json", "", body)
}

pub fn problem(status: &str, headers: &str, body: &str) -> String {
    response(status, "application/problem+json", headers, body)
}

pub const LLMS: &str = r##"{
  "url": "https://www.youtube.com/watch?v=abc",
  "kind": "video",
  "source": "generated",
  "mode": "full",
  "title": "A talk",
  "text": "# A talk\n\n> Notes.",
  "tokens": 1234,
  "pages": 1,
  "model": null,
  "cached": false,
  "generatedAt": "2026-11-14T00:00:00.000Z",
  "media": {
    "duration": 61.5,
    "hasVideo": true,
    "hasAudio": true,
    "thumbnail": "https://i.ytimg.com/vi/abc/hq.jpg",
    "downloads": ["mp4", "m4a"],
    "transcript": {"source": "captions", "segments": [{"start": 0, "end": 2.5, "text": "Hello"}]}
  }
}"##;

pub const CHECK: &str = r#"{
  "site": "https://llmstxt.org",
  "url": "https://llmstxt.org/llms.txt",
  "found": true,
  "status": 200,
  "title": "llms-txt",
  "tokens": 900,
  "sections": 2,
  "links": {"total": 30, "checked": 25, "broken": [{"url": "https://llmstxt.org/gone", "status": 404}]},
  "llmsFull": true,
  "issues": [
    {"level": "error", "code": "broken_link", "line": 12, "message": "Link answers 404."},
    {"level": "hint", "code": "future_check", "line": null, "message": "A level this crate doesn't know."}
  ],
  "summary": {"errors": 1, "warnings": 0},
  "checkedAt": "2026-11-14T00:00:00.000Z"
}"#;
