//! `--http-server`: a server on a free port, asked over plain TCP.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

const SOURCE: &str = "@startuml\nBob -> Alice : hello\n@enduml";
/// [`SOURCE`], encoded.
const CODE: &str = "SyfFKj2rKt3CoKnELR1Io4ZDoSa70000";

struct Server {
    child: Child,
    port: u16,
    /// Kept open, so that the server can go on writing to it.
    _stderr: BufReader<std::process::ChildStderr>,
}

impl Server {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rockuml"))
            .arg("--http-server:0:127.0.0.1:stop")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let mut line = String::new();
        stderr.read_line(&mut line).unwrap();
        let port = line
            .trim()
            .strip_prefix("webPort=")
            .unwrap_or_else(|| panic!("the port is announced: {line:?}"))
            .parse()
            .unwrap();
        Self {
            child,
            port,
            _stderr: stderr,
        }
    }

    /// The response's head and body.
    fn request(&self, request: &str) -> (String, Vec<u8>) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let split = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("a head");
        (
            String::from_utf8(response[..split].to_vec()).unwrap(),
            response[split + 4..].to_vec(),
        )
    }

    fn get(&self, path: &str) -> (String, Vec<u8>) {
        self.request(&format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n"))
    }

    fn post(&self, path: &str, body: &str) -> (String, Vec<u8>) {
        self.request(&format!(
            "POST {path} HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// What the command line draws of the source.
fn piped_svg(source: &str) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rockuml"))
        .args(["-tsvg", "-pipe"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap().stdout
}

#[test]
fn the_server_draws_encoded_and_posted_diagrams_like_the_command_line() {
    let server = Server::start();
    let expected = piped_svg(SOURCE);

    let (head, body) = server.get(&format!("/plantuml/svg/{CODE}"));
    assert!(head.starts_with("HTTP/1.1 200 OK"), "{head}");
    assert!(head.contains("Content-type: image/svg+xml"), "{head}");
    assert!(head.contains("X-PlantUML-Diagram-Width: "), "{head}");
    assert_eq!(body, expected);

    let (head, body) = server.post(
        "/render",
        r#"{"source": "Bob -> Alice : hello", "options": ["-tsvg"]}"#,
    );
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert_eq!(body, expected);

    let (head, body) = server.get(&format!("/png/{CODE}"));
    assert!(head.contains("Content-type: image/png"), "{head}");
    assert!(body.starts_with(b"\x89PNG"));
}

#[test]
fn the_server_reports_errors_and_what_it_cannot_do() {
    let server = Server::start();

    let (head, _) = server.get("/svg/IylFLqXAB0BYAW00");
    assert!(head.starts_with("HTTP/1.1 400 ERROR"), "{head}");
    assert!(
        head.contains("X-PlantUML-Diagram-Error: Syntax Error?"),
        "{head}"
    );
    assert!(head.contains("X-PlantUML-Diagram-Error-Line: 2"), "{head}");

    let (head, body) = server.post("/render", "not json");
    assert!(head.starts_with("HTTP/1.1 400 Bad Request"), "{head}");
    assert!(
        String::from_utf8(body)
            .unwrap()
            .starts_with("Error parsing request json")
    );

    let (head, _) = server.get(&format!("/txt/{CODE}"));
    assert!(head.starts_with("HTTP/1.1 501"), "{head}");

    let (head, _) = server.get("/elsewhere");
    assert!(head.starts_with("HTTP/1.1 302 Found"), "{head}");

    let (head, body) = server.get("/serverinfo");
    assert!(head.contains("Content-Type: application/json"), "{head}");
    assert_eq!(
        String::from_utf8(body).unwrap(),
        r#"{"version":"1.2026.8","PicoWebServer":true,"formats":["png","svg"]}"#
    );
}

#[test]
fn stopserver_stops_the_server() {
    let mut server = Server::start();
    let (head, _) = server.get("/stopserver");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(server.child.wait().unwrap().success());
}
