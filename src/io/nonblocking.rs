use mio::net::{TcpStream, TcpListener};
use mio::{Events, Interest, Poll, Token};
use std::time::Duration;

struct Connection {
    stream: TcpStream,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    state: State,
    keep_alive: bool,
}

enum State {
    ReadingHeader,
    ReadingBody,
    WritingHeader,
    WritingBody,
    ReadyToRespond,
    Closed
}

const LISTENER: Token = Token(0);

struct Reactor {
    poll: Poll,
    events: Events,
    listener: TcpListener,
    conns: slab::Slab<Connection>, // index → Connection
    // worker_rx: Receiver<(Token, Vec<u8>)>, // from thread pool back to loop (later)
}

impl Reactor {
    fn new(addr: &str) -> std::io::Result<Self> {
        let poll = Poll::new()?;
        let mut listener = TcpListener::bind(addr.parse().unwrap())?;
        // listener is nonblocking by default in mio
        poll.registry().register(
            &mut listener,
            LISTENER,
            Interest::READABLE,
        )?;
        Ok(Self {
            poll,
            events: Events::with_capacity(1024),
            listener,
            conns: slab::Slab::with_capacity(1024),
        })
    }
}