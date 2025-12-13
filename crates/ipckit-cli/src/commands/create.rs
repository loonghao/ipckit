//! Create command implementation

use super::{print_error, print_success};
use crate::ChannelType;
use ipckit::{LocalSocketListener, NamedPipe, SharedMemory};

pub fn create(
    channel_type: ChannelType,
    name: &str,
    size: usize,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match channel_type {
        ChannelType::Pipe => {
            if verbose {
                println!("Creating named pipe: {}", name);
            }
            let _pipe = NamedPipe::create(name)?;
            print_success(&format!("Created named pipe '{}'", name));
            println!("Waiting for client connection...");
            // Keep the pipe alive
            std::thread::park();
        }

        ChannelType::Shm => {
            if verbose {
                println!("Creating shared memory: {} (size: {} bytes)", name, size);
            }
            let _shm = SharedMemory::create(name, size)?;
            print_success(&format!(
                "Created shared memory '{}' ({} bytes)",
                name, size
            ));
            println!("Press Ctrl+C to close...");
            // Keep the shared memory alive
            std::thread::park();
        }

        ChannelType::Socket => {
            if verbose {
                println!("Creating local socket: {}", name);
            }
            let _listener = LocalSocketListener::bind(name)?;
            print_success(&format!("Created local socket '{}'", name));
            println!("Waiting for connections...");
            // Keep the socket alive
            std::thread::park();
        }

        ChannelType::File => {
            if verbose {
                println!("Creating file channel: {}", name);
            }
            let channel = ipckit::FileChannel::backend(name)?;
            print_success(&format!("Created file channel at '{}'", name));
            println!("Press Ctrl+C to close...");
            // Keep the channel alive
            drop(channel);
            std::thread::park();
        }

        ChannelType::Thread => {
            print_error("Thread channels cannot be created via CLI (they are in-process only)");
            return Err("Thread channels are in-process only".into());
        }
    }

    Ok(())
}
