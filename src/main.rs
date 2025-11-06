pub mod server;
pub mod thread_pool;
pub mod util;

pub mod http {
    pub mod request;
    pub mod response;
    pub mod headers;
}

pub mod io {
    pub mod nonblocking;
    pub mod file;
}

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

const ADDRESS: &str = "127.0.0.1:8080";

/// Entry point for the program
fn main() {
    let listener = match TcpListener::bind(ADDRESS) {
        Ok(listener) => listener,
        Err(e) => panic!("Unable to bind to {}: {}", ADDRESS, e),
    };

    // I'm not sure what number of threads is standard
    // However, the book used 4, so for now I will use 4 as well
    let thread_pool = thread_pool::ThreadPool::new(4);

    for stream in listener.incoming() {
        // Since .incoming() never returns a None I think .unwrap is okay for this.
        let stream = stream.unwrap();

        thread_pool.execute(|| {
            handle_connection(stream);
        });
    }
}

/// Handles a connection from a client.
///
/// Writes the desired HTML page into the TcpStream.
///
/// # Behavior
///
/// If the contents of the filename cannot be read, the error will
/// be logged into eprintln! Then write a new 500 Internal Server Error HTML page into the stream.
/// This should most likely be changed to not log into eprintln!
/// Such that we avoid leaking internal implementation to the user.
fn handle_connection(mut stream: TcpStream) {
    let buffer = BufReader::new(&mut stream);
    let request_line = buffer.lines().next().unwrap().unwrap();
    let (status, filename) = http_handler(&request_line);

    let html_page = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error reading file {filename}: {e}");
        String::from("<html><body><h1>500 Internal Server Error</h1></body></html>")
    });
    let length = html_page.len();

    let response =
        format!("{status}\r\nContent-Length: {length}\r\n\r\n{html_page}");

    stream.write_all(response.as_bytes()).unwrap();

}

/// Handles various HTML pages
///
/// # Example
///
/// HTML 200-page
/// HTML 404-page not found
///
/// As well as other pages which can be added cleanly into this method.
fn http_handler(response: &String) -> (&str, &str) {
    let (status, filename) = match &response[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "welcome.html"),
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };
    (status, filename)
}