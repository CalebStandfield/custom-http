use std::io::{ErrorKind, Write};
use std::net::TcpStream;
use crate::io;

/// Handles various HTML pages
///
/// # Example
///
/// HTML 200-page
/// HTML 403-page permission denied
/// HTML 404-page not found
/// HTML 500-page internal server error
///
/// As well as other pages which can be added cleanly into this method.
pub fn http_handler(stream: &TcpStream, response: String) {
    let (status, filename) = status_filename(response);

    let html_page = match io::file::read_file(&filename) {
        Ok(html_page) => html_page,
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => {
                    eprintln!("Error file not found {}: {}", filename, e);
                    let (status, filename) = status_filename(String::from("GET /404 HTTP/1.1"));
                    write_response(&stream, status, &filename);
                    return;
                }
                ErrorKind::PermissionDenied => {
                    eprintln!("Error permission denied{}: {}", filename, e);
                    let (status, filename) = status_filename(String::from("GET /403 HTTP/1.1"));
                    write_response(&stream, status, &filename);
                    return;
                }
                _ => {
                    eprintln!("Internal server error {}: {}", filename, e);
                    let (status, filename) = status_filename(String::from("GET /500 HTTP/1.1"));
                    write_response(&stream, status, &filename);
                    return;
                }
            }
        }
    };

    write_response(&stream, &status, &html_page);
}

fn status_filename<'a>(response: String) -> (&'a str, String) {
    let parts: Vec<&str> = response.split_whitespace().collect();
    if parts.len() < 2 {
        return ("HTTP/1.1 404", String::from("public/404.html"));
    }

    let path = parts[1];

    if path.contains("..") {
        ("HTTP/1.1 404", String::from("public/404.html"))
    } else if path == "/" {
        ("HTTP/1.1 200 OK", String::from("public/welcome.html"))
    } else {
        let mut path = String::from(path);
        path.push_str(".html");
        ("HTTP/1.1 200 OK", format!("public{}", path))
    }

}

fn write_response(mut stream: &TcpStream, status: &str, html_page: &str) {
    let length = html_page.len();

    let response =
        format!("{status}\r\nContent-Length: {length}\r\n\r\n{html_page}");

    stream.write_all(response.as_bytes()).unwrap();
}