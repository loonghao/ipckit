//! Example: Using IPC Channel for Message Passing in Rust
//!
//! Run with: cargo run --example channel_example

use ipckit::{IpcChannel, Result};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {
    #[serde(rename = "ping")]
    Ping { id: u32 },
    #[serde(rename = "pong")]
    Pong { id: u32 },
    #[serde(rename = "compute")]
    Compute { numbers: Vec<i32> },
    #[serde(rename = "result")]
    Result { value: i32 },
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "goodbye")]
    Goodbye,
}

fn main() -> Result<()> {
    let channel_name = format!("example_channel_{}", std::process::id());

    // Spawn server thread
    let server_channel_name = channel_name.clone();
    let server_handle = thread::spawn(move || -> Result<()> {
        println!("[Server] Creating channel: {}", server_channel_name);
        let mut channel = IpcChannel::<Message>::create(&server_channel_name)?;
        println!("[Server] Waiting for client...");

        channel.wait_for_client()?;
        println!("[Server] Client connected!");

        loop {
            let msg: Message = channel.recv()?;
            println!("[Server] Received: {:?}", msg);

            match msg {
                Message::Ping { id } => {
                    channel.send(&Message::Pong { id })?;
                }
                Message::Compute { numbers } => {
                    let value = numbers.iter().sum();
                    channel.send(&Message::Result { value })?;
                }
                Message::Exit => {
                    channel.send(&Message::Goodbye)?;
                    break;
                }
                _ => {}
            }
        }

        println!("[Server] Done!");
        Ok(())
    });

    // Wait for server to start
    thread::sleep(Duration::from_millis(500));

    // Client
    println!("[Client] Connecting to channel: {}", channel_name);
    let mut channel = IpcChannel::<Message>::connect(&channel_name)?;
    println!("[Client] Connected!");

    // Send ping
    channel.send(&Message::Ping { id: 1 })?;
    let response: Message = channel.recv()?;
    println!("[Client] Response: {:?}", response);

    // Send compute
    channel.send(&Message::Compute {
        numbers: vec![1, 2, 3, 4, 5],
    })?;
    let response: Message = channel.recv()?;
    println!("[Client] Response: {:?}", response);

    // Exit
    channel.send(&Message::Exit)?;
    let response: Message = channel.recv()?;
    println!("[Client] Response: {:?}", response);

    server_handle.join().unwrap()?;
    println!("\nDone!");

    Ok(())
}
