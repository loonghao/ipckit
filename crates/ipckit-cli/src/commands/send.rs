//! Send command implementation

use super::{channel_type_name, print_error, print_success};
use crate::ChannelType;
use ipckit::{LocalSocketStream, NamedPipe, SharedMemory};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub fn send(
    channel_type: ChannelType,
    name: &str,
    message: &str,
    file: Option<PathBuf>,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get message data
    let data = if let Some(path) = file {
        if verbose {
            println!("Reading message from file: {:?}", path);
        }
        fs::read(&path)?
    } else if message == "-" {
        if verbose {
            println!("Reading message from stdin...");
        }
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        buffer
    } else {
        message.as_bytes().to_vec()
    };

    if verbose {
        println!(
            "Sending {} bytes to {} '{}'",
            data.len(),
            channel_type_name(channel_type),
            name
        );
    }

    match channel_type {
        ChannelType::Pipe => {
            let mut pipe = NamedPipe::connect(name)?;
            pipe.write_all(&data)?;
            print_success(&format!("Sent {} bytes to pipe '{}'", data.len(), name));
        }

        ChannelType::Socket => {
            let mut stream = LocalSocketStream::connect(name)?;
            stream.write_all(&data)?;
            print_success(&format!("Sent {} bytes to socket '{}'", data.len(), name));
        }

        ChannelType::Shm => {
            let mut shm = SharedMemory::open(name)?;
            shm.write(0, &data)?;
            print_success(&format!(
                "Wrote {} bytes to shared memory '{}'",
                data.len(),
                name
            ));
        }

        ChannelType::File => {
            let channel = ipckit::FileChannel::backend(name)?;
            // Parse as JSON if possible, otherwise send as event
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&data) {
                channel.send_event("message", json)?;
            } else {
                channel.send_event(
                    "message",
                    serde_json::json!({
                        "data": String::from_utf8_lossy(&data)
                    }),
                )?;
            }
            print_success(&format!("Sent message to file channel '{}'", name));
        }

        ChannelType::Thread => {
            print_error("Thread channels cannot be used via CLI (they are in-process only)");
            return Err("Thread channels are in-process only".into());
        }
    }

    Ok(())
}
