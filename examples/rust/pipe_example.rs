//! Example: Using Named Pipes for IPC in Rust
//!
//! Run with: cargo run --example pipe_example

use ipckit::{NamedPipe, Result};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let pipe_name = format!("example_pipe_{}", std::process::id());

    // Spawn server thread
    let server_pipe_name = pipe_name.clone();
    let server_handle = thread::spawn(move || -> Result<()> {
        println!("[Server] Creating pipe: {}", server_pipe_name);
        let mut pipe = NamedPipe::create(&server_pipe_name)?;
        println!("[Server] Waiting for client...");

        pipe.wait_for_client()?;
        println!("[Server] Client connected!");

        // Receive message
        let mut buf = [0u8; 1024];
        let n = pipe.read(&mut buf)?;
        println!("[Server] Received: {}", String::from_utf8_lossy(&buf[..n]));

        // Send response
        let response = b"Hello from server!";
        pipe.write_all(response)?;
        println!("[Server] Sent: {}", String::from_utf8_lossy(response));

        Ok(())
    });

    // Wait a bit for server to start
    thread::sleep(Duration::from_millis(500));

    // Client
    println!("[Client] Connecting to pipe: {}", pipe_name);
    let mut pipe = NamedPipe::connect(&pipe_name)?;
    println!("[Client] Connected!");

    // Send message
    let message = b"Hello from client!";
    pipe.write_all(message)?;
    println!("[Client] Sent: {}", String::from_utf8_lossy(message));

    // Receive response
    let mut buf = [0u8; 1024];
    let n = pipe.read(&mut buf)?;
    println!("[Client] Received: {}", String::from_utf8_lossy(&buf[..n]));

    server_handle.join().unwrap()?;
    println!("\nDone!");

    Ok(())
}
