use crate::handle_connection;
use crate::thread_pool::ThreadPool;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::io;
use std::io::{Read, Write};
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
    Closed,
}

const LISTENER: Token = Token(0);

struct Reactor {
    poll: Poll,
    listener: TcpListener,
    conns: slab::Slab<Connection>,
    pool: ThreadPool,
}

impl Reactor {
    fn new(addr: &str) -> io::Result<Self> {
        let poll = Poll::new()?;
        let mut listener = TcpListener::bind(addr.parse().unwrap())?;
        let pool = ThreadPool::new(4);
        poll.registry()
            .register(&mut listener, LISTENER, Interest::READABLE)?;
        Ok(Self {
            poll,
            listener,
            conns: slab::Slab::with_capacity(1024),
            pool,
        })
    }
    fn event_loop(&mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(1024);

        loop {
            self.poll
                .poll(&mut events, Some(Duration::from_millis(1000)))?;

            for event in events.iter() {
                let token = event.token();

                if token == LISTENER {
                    self.accept_ready()?;
                } else {
                    self.handle_connection_event(token, event)?;
                }
            }
        }
    }
    fn accept_ready(&mut self) -> io::Result<()> {
        loop {
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    let conn = Connection {
                        stream,
                        read_buffer: Vec::new(),
                        write_buffer: Vec::new(),
                        state: State::ReadingHeader,
                        keep_alive: false,
                    };

                    // 2) Insert into slab, get index
                    let entry = self.conns.vacant_entry();
                    let key = entry.key();
                    let token = Token(key + 1); // 0 is reserved for LISTENER

                    // 3) Register this socket with poll
                    self.poll.registry().register(
                        &mut entry.insert(conn).stream,
                        token,
                        Interest::READABLE,
                    )?;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("accept error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
    fn handle_connection_event(&mut self, token: Token, event: &mio::event::Event) -> io::Result<()> {
        let idx = token.0 - 1;

        if event.is_readable() {
            if let Some(conn) = self.conns.get_mut(idx) {
                handle_readable(conn)?;

                if !conn.write_buffer.is_empty() {
                    self.poll.registry().reregister(
                        &mut conn.stream,
                        token,
                         // Set to WRITABLE if we have queued bytes to send
                        Interest::WRITABLE,
                    )?;
                }
            }
        }

        if event.is_writable() {
            if let Some(conn) = self.conns.get_mut(idx) {
                handle_writable(conn)?;

                if conn.write_buffer.is_empty() {
                    conn.state = State::Closed;
                    // TODO: deregister and remove from slab
                    // TODO: self.poll.registry().deregister(&mut conn.stream)?;
                    // TODO: self.conns.remove(idx);
                }
            }
        }

        Ok(())
    }
}

fn handle_writable(conn: &mut Connection) -> io::Result<()> {
    while !conn.write_buffer.is_empty() {
        let buf = &conn.write_buffer[..];

        match conn.stream.write(buf) {
            Ok(0) => {
                conn.state = State::Closed;
                break;
            }
            Ok(n) => {
                conn.write_buffer.drain(..n);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => {
                eprintln!("write error: {}", e);
                conn.state = State::Closed;
                break;
            }
        }
    }
    Ok(())
}

fn handle_readable(conn: &mut Connection) -> io::Result<()> {
    let mut buf = [0u8; 4096];

    loop {
        match conn.stream.read(&mut buf) {
            Ok(0) => {
                conn.state = State::Closed;
                break;
            }
            Ok(n) => {
                conn.read_buffer.extend_from_slice(&buf[..n]);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => {
                eprintln!("read error: {}", e);
                conn.state = State::Closed;
                break;
            }
        }
    }

    // TEMP TEST
    if !conn.read_buffer.is_empty() && conn.write_buffer.is_empty() {
        let body = b"Hello from mio\r\n";
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n",
            body.len()
        );
        conn.write_buffer.extend_from_slice(header.as_bytes());
        conn.write_buffer.extend_from_slice(body);
    }

    Ok(())
}

pub fn run(addr: &str) -> io::Result<()> {
    let mut reactor = Reactor::new(addr)?;
    reactor.event_loop()?;

    Ok(())
}
