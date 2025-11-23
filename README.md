# Custom HTTP Server in Rust

This project is a custom HTTP/1.1 server written from scratch in Rust.  
The goal is to explore how a real web server works internally by implementing the full stack manually — from socket management to concurrency and asynchronous I/O.

It builds directly on what I learned from *The Rust Programming Language* book and extends those ideas into a larger, real-world system.
Especially concurrency and asynchronous programming. 

---

## Overview

The server is designed around three main goals:
1. Use **concurrency primitives** from Rust’s standard library to handle multiple connections safely.
2. Build a **custom thread pool** to manage worker threads and prevent thread-per-connection overhead.
3. Add **asynchronous I/O** for low-latency, non-blocking file serving.

The final result is a small but capable HTTP server that can serve files, handle concurrent requests, learn how modern web servers achieve performance through parallelism and event-driven I/O, and while practicing Rust.

---

## Architecture
forgive me if this looks like a child made it. 

    src/ 

      main.rs
      
      server.rs
      
      thread_pools.rs
      
      utils.rs
      
      io/
      
        file.rs
        
        nonblocking.rs
        
      https/
      
        headers.rs
        
        request.rs
        
        response.rs

Each component is written to be clear and self-contained, without `mod.rs` files (mostly because I hate them and think they are outdated, but that's a discussion for later). The design is meant to keep things separate and thus navigable :)

---

## Technical Focus

- **Thread Pool Architecture** – Fixed-size pool with a job queue for efficient scheduling.  
- **Event-Driven I/O** – Uses `mio` for readiness-based I/O instead of blocking sockets.  
- **HTTP Parsing** – Minimal but compliant request/response handling.  
- **File Serving** – Efficient static file delivery with buffering and MIME detection.  

---

## Stack

- Rust 1.91+ required (I actually don't know what the minimum version is, but 1.91 is what I'm on) 
- `std::thread`, `std::sync`
- [`mio`](https://crates.io/crates/mio)
- [`mime_guess`](https://crates.io/crates/mime_guess)

---

## Purpose

This is in no way shape or form meant to replace an existing framework or library — it’s just an exercise in understanding what happens beneath them. By implementing concurrency, scheduling, and I/O from first principles, I'll be able to see how performance and correctness interact in low-level systems programming, which is currently what I'm interested in and would like a bit more practice in.
