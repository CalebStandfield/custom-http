//! Handles reading files from the disk and determining their MIME types.
//!
//! This module provides utilities for loading static files from the
//! `/public` directory, as well as helpers for detecting and returning
//! appropriate MIME types. All functions here are synchronous and
//! blocking. Future implementations may be asynchronous.
use crate::io;
use mime_guess::{from_path, mime};
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;

/// An enumeration representing different types of error pages that can be displayed in an application.
///
/// This enum is typically used to categorize errors and provide appropriate error pages
/// or messages to the users. Each variant corresponds to a specific error scenario.
///
/// Variants:
/// - `NotFound`: Indicates that the requested resource could not be found (HTTP 404).
/// - `PermissionDenied`: Indicates that the user does not have the necessary permissions
///   to access the requested resource (HTTP 403).
/// - `InternalServerError`: Indicates that an unexpected server error has occurred (HTTP 500).
///
/// Use this enum to clearly define and handle error scenarios in your application.
enum ErrorPage {
    NotFound,
    PermissionDenied,
    InternalServerError,
}

/// Returns the path to the error page for the given error page variant.
impl ErrorPage {
    /// Returns the file path of the HTML page corresponding to the error type.
    ///
    /// This function maps the current `ErrorPage` variant to its associated
    /// HTML file path, which represents the error page to be displayed.
    ///
    /// # Returns
    ///
    /// A `String` containing the file path of the error page.
    ///
    /// # Variants
    ///
    /// * `ErrorPage::NotFound` - Returns `"public/404.html"`, the path for the 404 Not Found error page.
    /// * `ErrorPage::PermissionDenied` - Returns `"public/403.html"`, the path for the 403 Permission Denied error page.
    /// * `ErrorPage::InternalServerError` - Returns `"public/500.html"`, the path for the 500 Internal Server Error page.
    ///
    /// # Example
    ///
    /// ```
    /// let error = ErrorPage::NotFound;
    /// assert_eq!(error.path(), "public/404.html");
    /// ```
    fn path(&self) -> String {
        match self {
            ErrorPage::NotFound => String::from("public/404.html"),
            ErrorPage::PermissionDenied => String::from("public/403.html"),
            ErrorPage::InternalServerError => String::from("public/500.html"),
        }
    }

    /// Returns the HTTP response status line as a `String` corresponding to the error type.
    ///
    /// This method matches the current variant of the `ErrorPage` enum and returns a properly formatted
    /// HTTP response status line as a `String`.
    ///
    /// # Variants
    ///
    /// - `ErrorPage::NotFound`: Returns `"HTTP/1.1 404 NOT FOUND"`
    /// - `ErrorPage::PermissionDenied`: Returns `"HTTP/1.1 403 PERMISSION DENIED"`
    /// - `ErrorPage::InternalServerError`: Returns `"HTTP/1.1 500 INTERNAL SERVER ERROR"`
    ///
    /// # Examples
    ///
    /// ```
    /// let error = ErrorPage::NotFound;
    /// assert_eq!(error.status(), "HTTP/1.1 404 NOT FOUND");
    /// ```
    fn status(&self) -> String {
        match self {
            ErrorPage::NotFound => String::from("HTTP/1.1 404 NOT FOUND"),
            ErrorPage::PermissionDenied => String::from("HTTP/1.1 403 PERMISSION DENIED"),
            ErrorPage::InternalServerError => String::from("HTTP/1.1 500 INTERNAL SERVER ERROR"),
        }
    }
}

/// Represents an HTTP response.
///
/// The `HttpResponse` struct holds information about an HTTP response,
/// including its status, content type, and body.
///
/// # Fields
/// - `status` (*String*): The HTTP status code and description (e.g., "200 OK", "404 Not Found").
/// - `content_type` (*String*): The MIME type of the content being returned (e.g., "text/html", "application/json").
/// - `body` (*Body*): The actual data being sent as part of the response. The `Body` type represents the content of the response and may encapsulate text, binary data, etc.
///
/// # Example
/// ```
/// let response = HttpResponse {
///     status: String::from("200 OK"),
///     content_type: String::from("application/json"),
///     body: Body::Text(String::from("{\"key\": \"value\"}")),
/// };
/// ```
struct HttpResponse {
    status: String,
    content_type: String,
    body: Body,
}

/// An `enum` representing the possible types of body content.
///
/// The `Body` enum is used to encapsulate different formats of data that can be
/// stored or transmitted in an application, particularly useful in contexts like
/// HTTP bodies or message payloads.
///
/// # Variants
///
/// - `Text(String)`
///   Represents the body content as a plain text string.
///   Typically used for textual data such as JSON, XML, or plain text.
///
/// - `Binary(Vec<u8>)`
///   Represents the body content as binary data.
///   Useful for handling non-text data such as images, files, or other raw byte streams.
///
/// # Examples
///
/// ```rust
/// // A textual body containing a JSON string
/// let text_body = Body::Text(String::from("{\"key\": \"value\"}"));
///
/// // A binary body containing raw byte data
/// let binary_body = Body::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF]);
/// ```
enum Body {
    Text(String),
    Binary(Vec<u8>),
}

/// Handles an incoming HTTP connection by constructing and sending an HTTP response.
///
/// # Arguments
///
/// * `stream` - A reference to the `TcpStream` representing the client's connection.
/// * `response` - A `String` containing the content to be included in the HTTP response body.
///
/// # Functionality
///
/// 1. Creates an `HttpResponse` object by processing the provided `response` string with `create_http_response`.
/// 2. Writes the generated HTTP response to the provided `TcpStream` using the `write_response` function.
///
/// # Example
///
/// ```
/// use std::net::{TcpListener, TcpStream};
///
/// fn main() {
///     let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
///
///     for stream in listener.incoming() {
///         let stream = stream.unwrap();
///         http_handler(&stream, "Hello, World!".to_string());
///     }
/// }
/// ```
///
/// # Dependencies
///
/// This function relies on two helper functions:
/// - `create_http_response` - To construct an `HttpResponse` object from the provided response content.
/// - `write_response` - To send the `HttpResponse` over the `TcpStream`.
///
/// # Notes
///
/// - Ensure that the `response` string is properly formatted to be suitable for inclusion in an HTTP response.
/// - It is assumed that `create_http_response` and `write_response` are implemented elsewhere in the codebase.
///
/// # Errors
///
/// If the `write_response` function fails, the error will not be handled in this function.
/// The caller of this function may want to log or handle any network-related errors outside of this context.
pub fn http_handler(stream: &TcpStream, response: String) {
    let http_response: HttpResponse = create_http_response(response);
    write_response(&stream, http_response);
}

/// Creates an HTTP response based on the given file path or error page response.
///
/// This function attempts to generate an `HttpResponse` object by first determining
/// the status code and filename through the helper function `status_filename`. It then
/// reads the file's content, assigns the appropriate MIME type, and prepares the response
/// with the content as either text or binary data, depending on the file type and encoding.
///
/// If the file cannot be read (e.g., it does not exist or there is a permission issue), the
/// function falls back to using an "Internal Server Error" page. In such a case, it retrieves
/// the fallback error page path, reads its content, and updates the MIME type accordingly.
///
/// # Parameters
/// - `response`: A `String` representing a response indicator, which is used to determine
///   the HTTP status and the associated file path that should be served.
///
/// # Returns
/// - An `HttpResponse` containing:
///   - `status`: The HTTP status code as a `String`.
///   - `content_type`: The MIME type of the response content as a `String`.
///   - `body`: The response body, which is either text or binary data.
///
/// # Errors
/// - If both the requested file and the fallback error file cannot be read,
///   this function will panic due to the `unwrap()` call on reading the error file.
///
/// # Example
/// ```
/// let response = create_http_response(String::from("/index.html"));
/// println!("HTTP Status: {}", response.status);
/// println!("Content Type: {}", response.content_type);
/// ```
///
/// # Dependencies
/// - This function makes use of external helper functions such as
///   - `status_filename(response: String) -> (String, String)`: Determines the HTTP status
///     and corresponding file path.
///   - `from_path(path: &str) -> Mime`: Determines the MIME type of a file based on its path.
///   - `io::file::read_file_bytes(path: &str) -> Result<Vec<u8>, IoError>`: Reads file content
///     as a byte vector.
///   - `ErrorPage::InternalServerError`: Contains the status code and path for the internal server
///     error fallback page.
///
/// # Notes
/// - If the file's MIME type is `text/*`, the function attempts to decode the file contents
///   as UTF-8. If decoding fails, the content is returned as binary data.
/// - The function logs an error message to `stderr` if the requested file cannot be read.
///
/// # Warning
/// - Use caution with the `unwrap()` call when reading the fallback error file, as it will cause
///   the program to panic in case of an unrecoverable error.
fn create_http_response(response: String) -> HttpResponse {
    let (mut status, mut filename) = status_filename(response);
    let mut mime = from_path(&filename).first_or_octet_stream();
    let bytes = match io::file::read_file_bytes(&filename) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading file {}: {}", filename, e);
            status = String::from(ErrorPage::InternalServerError.status());
            filename = String::from(ErrorPage::InternalServerError.path());
            mime = from_path(&filename).first_or_octet_stream();
            io::file::read_file_bytes(&filename).unwrap()
        }
    };

    let body = if mime.type_() == mime::TEXT {
        // Try for text first, if that fails, fall back to binary
        match String::from_utf8(bytes.clone()) {
            Ok(text) => Body::Text(text),
            Err(_) => Body::Binary(bytes),
        }
    } else {
        Body::Binary(bytes)
    };

    HttpResponse {
        status,
        content_type: mime.to_string(),
        body,
    }
}

/// Returns the status and file path of the inputted response string.
///
/// Handles both the 404 and 403 logic by checking the path exists and determining
/// if it tries to access improper files.
///
/// # Parameters
/// - `response`: the `String` to parse for the status and filename/path.
///
/// # Returns
/// - `(String, String)`: The status and filepath.
fn status_filename(response: String) -> (String, String) {
    let parts: Vec<&str> = response.split_whitespace().collect();
    if parts.len() < 2 {
        return (
            String::from("HTTP/1.1 404"),
            String::from("public/404.html"),
        );
    }

    let path = parts[1];

    if path.contains("..") {
        (
            String::from(ErrorPage::PermissionDenied.status()),
            String::from(ErrorPage::PermissionDenied.path()),
        )
    } else if path == "/" {
        (
            // Landing page if no path is specified
            String::from("HTTP/1.1 200 OK"),
            String::from("public/welcome.html"),
        )
    } else {
        let mut path = String::from(path);
        path.push_str(".html");
        if !Path::new(&path).exists() {
            return (
                String::from(ErrorPage::NotFound.status()),
                String::from(ErrorPage::NotFound.path()),
            );
        }
        (String::from("HTTP/1.1 200 OK"), format!("public{}", path))
    }
}

/// Writes an HTTP response to the provided TCP stream.
///
/// This function serializes the HTTP headers and body into bytes
/// and sends them over the given `TcpStream`. It ensures that
/// `Content-Length` and `Content-Type` are properly set based on
/// the `HttpResponse` struct.
///
/// # Parameters
/// - `stream`: The open `TcpStream` representing the client connection.
/// - `http_response`: The HTTP response to send, including status,
///   headers, and body.
fn write_response(mut stream: &TcpStream, http_response: HttpResponse) {
    let status = http_response.status;
    let mime = http_response.content_type;
    let (length, body_bytes): (usize, &[u8]) = match &http_response.body {
        Body::Text(text) => (text.len(), text.as_bytes()),
        Body::Binary(binary) => (binary.len(), binary),
    };

    let header = format!("{status}\r\nContent-Length: {length}\r\nContent-Type: {mime}\r\n\r\n");
    stream.write_all(header.as_bytes()).unwrap();
    stream.write_all(body_bytes).unwrap();
}
