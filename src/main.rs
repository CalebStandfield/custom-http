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

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use crate::io::nonblocking;

const ADDRESS: &str = "127.0.0.1:8080";

/// Entry point for the program
fn main() {
    nonblocking::run(ADDRESS).expect("TODO: Match errors");
}

/// Handles a connection from a client.
///
/// Writes the desired page into the TcpStream.
fn handle_connection(mut stream: TcpStream) {
    let buffer = BufReader::new(&mut stream);
    let request_line = buffer.lines().next().unwrap().unwrap();
    let response = http::response::http_handler(request_line);
}