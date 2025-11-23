use crate::io::nonblocking;

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

const ADDRESS: &str = "127.0.0.1:8080";

/// Entry point for the program
fn main() {
    nonblocking::run(ADDRESS).expect("TODO: Match errors");
}