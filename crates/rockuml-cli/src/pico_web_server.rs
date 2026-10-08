//! `--http-server[:port[:address]][:stop]`: renders diagrams over HTTP, so that editors' PlantUML plugins can
//! use rockuml as their server (PlantUML's `PicoWebServer`, `ReceivedHTTPRequest` and `RenderRequest`).
//!
//! Unlike PlantUML, `/stopserver` (with `stop`) really stops the server, ASCII-art and keyword-list requests
//! answer that they are not ported, and responses leave out PlantUML's description, title, donation and quote
//! headers.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use rockuml::diagram::{DiagramError, ExportedImage};
use rockuml::fonts::FontRegistry;
use rockuml::json::{JsonObject, JsonValue};

use crate::cli_options::CliOptions;
use crate::cli_parsed::CliParsed;
use crate::crash;
use crate::file_format::FileFormat;
use crate::run::{STACK_SIZE, Settings};

/// What PlantUML redirects unknown requests to: an image of `Bob -> Alice : hello`.
const WELCOME: &str = "/plantuml/png/oqbDJyrBuGh8ISmh2VNrKGZ8JCuFJqqAJYqgIotY0aefG5G00000";

/// Serves requests until stopped; each connection is handled on its own thread.
pub(crate) fn start_server(
    port: u16,
    bind_address: Option<&str>,
    enable_stop: bool,
    fonts: &Arc<FontRegistry>,
) -> Result<(), String> {
    let address = bind_address.unwrap_or("0.0.0.0");
    let listener = TcpListener::bind((address, port))
        .map_err(|error| format!("cannot listen on {address}:{port}: {error}"))?;
    let local_port = listener
        .local_addr()
        .map_err(|error| format!("cannot listen on {address}:{port}: {error}"))?
        .port();
    // A closed stderr must not stop the server.
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "webPort={local_port}");
    let _ = writeln!(stderr, "webAddress={address}");
    for connection in listener.incoming() {
        let Ok(stream) = connection else {
            continue;
        };
        let fonts = fonts.clone();
        // A connection that cannot get a thread is dropped, as a busy server would.
        let _ = std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || serve(stream, enable_stop, &fonts));
    }
    Ok(())
}

/// Why a request fails: the client's fault (400), a diagram rockuml cannot draw yet (501), or the server's (500).
enum HttpError {
    BadRequest(String),
    NotImplemented(String),
    Internal(String),
}

fn serve(mut stream: TcpStream, enable_stop: bool, fonts: &Arc<FontRegistry>) {
    let result = ReceivedHttpRequest::from_stream(&mut BufReader::new(&stream))
        .and_then(|request| respond(&request, enable_stop, fonts));
    let response = match result {
        Ok(Response::Stop(response)) => {
            let _ = stream.write_all(&response);
            // Closed in order before exiting, which would otherwise reset the connection under the response.
            let _ = stream.shutdown(std::net::Shutdown::Write);
            std::process::exit(0);
        }
        Ok(Response::Send(response)) => response,
        Err(HttpError::BadRequest(message)) => error_response("400 Bad Request", &message),
        Err(HttpError::NotImplemented(message)) => error_response("501 Not Implemented", &message),
        Err(HttpError::Internal(message)) => error_response("500 Internal Server Error", &message),
    };
    // The client may have gone; there is nobody left to tell.
    let _ = stream.write_all(&response);
    let _ = stream.flush();
}

enum Response {
    Send(Vec<u8>),
    /// Sent, then the server stops.
    Stop(Vec<u8>),
}

fn respond(
    request: &ReceivedHttpRequest,
    enable_stop: bool,
    fonts: &Arc<FontRegistry>,
) -> Result<Response, HttpError> {
    let path = request.path.as_str();
    let under =
        |prefix: &str| path.starts_with(prefix) || path.starts_with(&format!("/plantuml{prefix}"));
    if request.method == "GET" {
        for (prefix, format) in [("/png/", FileFormat::Png), ("/svg/", FileFormat::Svg)] {
            if under(prefix)
                && let Some(response) = handle_get(path, format, fonts)?
            {
                return Ok(Response::Send(response));
            }
        }
        if under("/txt/") || under("/utxt/") {
            return Ok(Response::Send(not_ported("ASCII-art output")));
        }
        if under("/serverinfo") {
            return Ok(Response::Send(handle_info()));
        }
        if path.starts_with("/language") {
            return Ok(Response::Send(not_ported("the keyword list")));
        }
        if enable_stop && under("/stopserver") {
            return Ok(Response::Stop(text_response(
                "200",
                "text/html",
                "<html>Stopping...</html>",
            )));
        }
    } else if request.method == "POST" && path == "/render" {
        return handle_render_request(request, fonts).map(Response::Send);
    }
    Ok(Response::Send(
        [
            "HTTP/1.1 302 Found\r\n".to_owned(),
            format!("Location: {WELCOME}\r\n"),
            "\r\n".to_owned(),
        ]
        .concat()
        .into_bytes(),
    ))
}

/// The image of the diagram encoded in the last part of the path, or nothing for a path without one.
fn handle_get(
    path: &str,
    format: FileFormat,
    fonts: &Arc<FontRegistry>,
) -> Result<Option<Vec<u8>>, HttpError> {
    let compressed = &path[path.rfind('/').map_or(0, |slash| slash + 1)..];
    let source = rockuml::url_code::decode(compressed)
        .map_err(|_| HttpError::BadRequest(format!("not a PlantUML code: {compressed}")))?;
    let settings = request_settings(&[], format, fonts)?;
    let Some(rendered) = render(&source, &settings)? else {
        return Ok(None);
    };
    let return_code = if rendered.error.is_some() {
        "400 ERROR"
    } else {
        "200 OK"
    };
    Ok(Some(diagram_response(return_code, format, &rendered)))
}

/// `POST /render` with `{"source": "...", "options": ["-tsvg", ...]}`.
fn handle_render_request(
    request: &ReceivedHttpRequest,
    fonts: &Arc<FontRegistry>,
) -> Result<Vec<u8>, HttpError> {
    if request.body.is_empty() {
        return Err(HttpError::BadRequest("No request body".to_owned()));
    }
    let (options, source) = render_request_from_json(&String::from_utf8_lossy(&request.body))
        .map_err(|message| {
            HttpError::BadRequest(format!("Error parsing request json: {message}"))
        })?;
    let source = if source.starts_with("@start") {
        source
    } else {
        format!("@startuml\n{source}\n@enduml")
    };
    let settings = request_settings(&options, FileFormat::Png, fonts)?;
    let format = settings.format;
    let rendered = render(&source, &settings)?.ok_or_else(|| {
        HttpError::BadRequest("No valid @start/@end found, please check the version".to_owned())
    })?;
    Ok(diagram_response("200", format, &rendered))
}

/// The request's options, read like a command line's (`RenderRequest.fromJson`).
fn render_request_from_json(json: &str) -> Result<(Vec<String>, String), String> {
    let parsed = rockuml::json::parse(json).map_err(|error| error.to_string())?;
    let JsonValue::Object(object) = parsed else {
        return Err("not an object".to_owned());
    };
    let options = match object.get("options") {
        None => Vec::new(),
        Some(JsonValue::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("an option is not a string")
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("options are not an array".to_owned()),
    };
    let source = object
        .get("source")
        .and_then(JsonValue::as_str)
        .ok_or("no source")?
        .to_owned();
    Ok((options, source))
}

/// The settings of a request: its options, or `format` alone; the server's fonts.
fn request_settings(
    options: &[String],
    format: FileFormat,
    fonts: &Arc<FontRegistry>,
) -> Result<Settings, HttpError> {
    let mut arguments = options.to_vec();
    if arguments.is_empty() {
        arguments.push(format!("-t{}", format_name(format)));
    }
    let flags = CliParsed::parse(arguments).map_err(|error| HttpError::BadRequest(error.0))?;
    let options = CliOptions::new(flags, None).map_err(|error| HttpError::BadRequest(error.0))?;
    let mut settings = Settings::of_options(options).map_err(HttpError::BadRequest)?;
    settings.fonts = fonts.clone();
    Ok(settings)
}

fn format_name(format: FileFormat) -> &'static str {
    match format {
        FileFormat::Svg => "svg",
        _ => "png",
    }
}

struct Rendered {
    image: ExportedImage,
    error: Option<DiagramError>,
}

/// The first diagram of the source as an image; nothing when the source has no diagram.
fn render(source: &str, settings: &Settings) -> Result<Option<Rendered>, HttpError> {
    let image_format = settings.format.image_format().ok_or_else(|| {
        HttpError::BadRequest("the server draws images only: png, svg or debug".to_owned())
    })?;
    crash::catch(|| {
        let blocks = crate::pipe::preprocess(source, settings);
        let Some(block) = blocks.first() else {
            return Ok(None);
        };
        let diagram = rockuml::diagram::create(block, &settings.host)
            .map_err(|not_ported| HttpError::NotImplemented(not_ported.to_string()))?;
        let image = rockuml::diagram::export_image(
            diagram.as_ref(),
            0,
            image_format,
            settings.options.metadata(),
            &settings.fonts,
            &settings.host,
        )
        .map_err(|not_ported| HttpError::NotImplemented(not_ported.to_string()))?;
        Ok(Some(Rendered {
            image,
            error: diagram.error(),
        }))
    })
    .unwrap_or_else(|message| Err(HttpError::Internal(message)))
}

fn diagram_response(return_code: &str, format: FileFormat, rendered: &Rendered) -> Vec<u8> {
    let mut head = vec![
        format!("HTTP/1.1 {return_code}"),
        "Cache-Control: no-cache".to_owned(),
        server_header(),
        "Access-Control-Allow-Origin: *".to_owned(),
        format!("Content-type: {}", format.mime_type()),
        format!("Content-length: {}", rendered.image.data.len()),
        format!("X-PlantUML-Diagram-Width: {}", rendered.image.width),
        format!("X-PlantUML-Diagram-Height: {}", rendered.image.height),
    ];
    if let Some(error) = &rendered.error {
        head.push(format!("X-PlantUML-Diagram-Error: {}", error.message));
        head.push(format!("X-PlantUML-Diagram-Error-Line: {}", error.line + 1));
    }
    let mut response = lines(&head);
    response.extend_from_slice(&rendered.image.data);
    response
}

fn handle_info() -> Vec<u8> {
    let mut info = JsonObject::new();
    info.add(
        "version",
        JsonValue::String(rockuml::PLANTUML_VERSION.to_owned()),
    );
    info.add("PicoWebServer", JsonValue::Bool(true));
    info.add(
        "formats",
        JsonValue::Array(vec![
            JsonValue::String("png".to_owned()),
            JsonValue::String("svg".to_owned()),
        ]),
    );
    text_response(
        "200",
        "application/json",
        &JsonValue::Object(info).to_string(),
    )
}

fn not_ported(what: &str) -> Vec<u8> {
    error_response(
        "501 Not Implemented",
        &format!("{what} is not ported to rockuml yet"),
    )
}

fn text_response(return_code: &str, content_type: &str, body: &str) -> Vec<u8> {
    let mut response = lines(&[
        format!("HTTP/1.1 {return_code}"),
        "Cache-Control: no-cache".to_owned(),
        server_header(),
        format!("Content-Type: {content_type}"),
    ]);
    response.extend_from_slice(body.as_bytes());
    response
}

fn error_response(status: &str, message: &str) -> Vec<u8> {
    let mut response = lines(&[
        format!("HTTP/1.1 {status}"),
        "Content-type: text/plain".to_owned(),
        format!("Content-length: {}", message.len()),
    ]);
    response.extend_from_slice(message.as_bytes());
    response
}

fn server_header() -> String {
    format!(
        "Server: rockuml {} (PlantUML {} compatible)",
        env!("CARGO_PKG_VERSION"),
        rockuml::PLANTUML_VERSION
    )
}

/// The header lines, each ended with CRLF, then the empty line before the body.
fn lines(head: &[String]) -> Vec<u8> {
    let mut text = String::new();
    for line in head {
        text.push_str(line);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    text.into_bytes()
}

/// A request's method, path and body; other headers are skipped (`ReceivedHTTPRequest`).
struct ReceivedHttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

impl ReceivedHttpRequest {
    fn from_stream(input: &mut impl BufRead) -> Result<Self, HttpError> {
        const CONTENT_LENGTH_HEADER: &str = "content-length: ";
        let request_line = read_line(input)?;
        let tokens: Vec<&str> = request_line.split_whitespace().collect();
        let [method, path, _version] = tokens.as_slice() else {
            return Err(HttpError::BadRequest("Bad request line".to_owned()));
        };
        let mut content_length = 0;
        loop {
            let line = read_line(input)?;
            if line.is_empty() {
                break;
            }
            if let Some(length) = line
                .to_lowercase()
                .strip_prefix(CONTENT_LENGTH_HEADER)
                .map(|length| length.trim().to_owned())
            {
                content_length = length
                    .parse()
                    .map_err(|_| HttpError::BadRequest("Invalid content length".to_owned()))?;
            }
        }
        let mut body = vec![0; content_length];
        input
            .read_exact(&mut body)
            .map_err(|_| HttpError::BadRequest("Body too short".to_owned()))?;
        Ok(Self {
            method: method.to_uppercase(),
            path: (*path).to_owned(),
            body,
        })
    }
}

/// A line up to `\n`, without its `\r`; bytes are taken as Latin-1 characters, as PlantUML does.
fn read_line(input: &mut impl BufRead) -> Result<String, HttpError> {
    let mut bytes = Vec::new();
    input
        .read_until(b'\n', &mut bytes)
        .map_err(|error| HttpError::Internal(error.to_string()))?;
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    Ok(bytes.iter().map(|&byte| char::from(byte)).collect())
}
