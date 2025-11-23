use crate::http::response;
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

                    // 3) Register this socket with 'poll'
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

    fn handle_connection_event(
        &mut self,
        token: Token,
        event: &mio::event::Event,
    ) -> io::Result<()> {
        let idx = token.0 - 1;

        if event.is_readable() {
            if let Some(conn) = self.conns.get_mut(idx) {
                self.handle_readable(idx, token)?;
            }
        }

        if event.is_writable() {
            if let Some(conn) = self.conns.get_mut(idx) {
                self.handle_writable(idx, token)?;
            }
        }

        Ok(())
    }

    fn handle_writable(&mut self, idx: usize, token: Token) -> io::Result<()> {
        let conn = match self.conns.get_mut(idx) {
            Some(conn) => conn,
            None => return Ok(()),
        };

        while !conn.write_buffer.is_empty() {
            let buf = &conn.write_buffer[..];

            match conn.stream.write(buf) {
                Ok(0) => {
                    conn.state = State::Closed;
                    self.poll.registry().deregister(&mut conn.stream)?;
                    self.conns.remove(idx);
                    return Ok(());
                }
                Ok(n) => {
                    self.poll.registry().reregister(
                        &mut conn.stream,
                        token,
                        // Set to WRITABLE if we have queued bytes to send
                        Interest::READABLE,
                    )?;
                    conn.write_buffer.drain(..n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    eprintln!("write error: {}", e);
                    conn.state = State::Closed;
                    self.poll.registry().deregister(&mut conn.stream)?;
                    self.conns.remove(idx);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    fn handle_readable(&mut self, idx: usize, token: Token) -> io::Result<()> {
        let conn = match self.conns.get_mut(idx) {
            Some(conn) => conn,
            None => return Ok(()),
        };

        let mut buf = [0u8; 4096];

        loop {
            match conn.stream.read(&mut buf) {
                Ok(0) => {
                    conn.state = State::Closed;
                    self.poll.registry().deregister(&mut conn.stream)?;
                    self.conns.remove(idx);
                    return Ok(());
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
                    self.poll.registry().deregister(&mut conn.stream)?;
                    self.conns.remove(idx);
                    return Ok(());
                }
            }
        }

        if !conn.read_buffer.is_empty() {
            let header_end = match conn.read_buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                Some(pos) => pos + 4,
                None => return Ok(()),
            };

            let header_bytes = &conn.read_buffer[..header_end];
            let header_text = String::from_utf8_lossy(header_bytes);

            let mut lines = header_text.lines();
            let request_line = match lines.next() {
                Some(line) => line.to_string(),
                None => return Ok(()), // TODO: later 400 this
            };

            let bytes = response::http_handler(request_line);

            conn.write_buffer.extend_from_slice(&bytes);

            self.poll
                .registry()
                .reregister(&mut conn.stream, token, Interest::WRITABLE)?;
        }

        Ok(())
    }
}

pub fn run(addr: &str) -> io::Result<()> {
    let mut reactor = Reactor::new(addr)?;
    reactor.event_loop()?;

    Ok(())
}
