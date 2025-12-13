//! Listen command implementation

use super::{channel_type_name, format_output, print_error, print_info, print_success};
use crate::{ChannelType, OutputFormat};
use ipckit::{LocalSocketListener, NamedPipe, SharedMemory};
use std::io::Read;
use std::time::Duration;

pub fn listen(
    channel_type: ChannelType,
    name: &str,
    format: OutputFormat,
    timeout_ms: u64,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    print_info(&format!(
        "Listening on {} '{}'...",
        channel_type_name(channel_type),
        name
    ));

    match channel_type {
        ChannelType::Pipe => {
            let mut pipe = NamedPipe::create(name)?;
            if verbose {
                println!("Named pipe created, waiting for client...");
            }
            pipe.wait_for_client()?;
            print_success("Client connected");

            loop {
                let mut buffer = vec![0u8; 4096];
                match pipe.read(&mut buffer) {
                    Ok(0) => {
                        print_info("Connection closed");
                        break;
                    }
                    Ok(n) => {
                        let data = &buffer[..n];
                        println!("{}", format_output(data, format));
                    }
                    Err(e) => {
                        print_error(&format!("Read error: {}", e));
                        break;
                    }
                }
            }
        }

        ChannelType::Socket => {
            let listener = LocalSocketListener::bind(name)?;
            if verbose {
                println!("Socket bound, waiting for connections...");
            }

            loop {
                match listener.accept() {
                    Ok(mut stream) => {
                        print_success("Client connected");
                        loop {
                            let mut buffer = vec![0u8; 4096];
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    print_info("Connection closed");
                                    break;
                                }
                                Ok(n) => {
                                    let data = &buffer[..n];
                                    println!("{}", format_output(data, format));
                                }
                                Err(e) => {
                                    print_error(&format!("Read error: {}", e));
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        print_error(&format!("Accept error: {}", e));
                        break;
                    }
                }
            }
        }

        ChannelType::Shm => {
            let shm = SharedMemory::open(name)?;
            if verbose {
                println!("Shared memory opened");
            }

            // Poll shared memory for changes
            let poll_interval = if timeout_ms > 0 {
                Duration::from_millis(timeout_ms.min(100))
            } else {
                Duration::from_millis(100)
            };

            let mut last_data: Vec<u8> = Vec::new();
            loop {
                let data = shm.read(0, shm.size())?;
                if data != last_data {
                    println!("{}", format_output(&data, format));
                    last_data = data;
                }
                std::thread::sleep(poll_interval);
            }
        }

        ChannelType::File => {
            let mut channel = ipckit::FileChannel::frontend(name)?;
            if verbose {
                println!("File channel opened");
            }

            let poll_interval = if timeout_ms > 0 {
                Duration::from_millis(timeout_ms.min(100))
            } else {
                Duration::from_millis(100)
            };

            loop {
                match channel.recv() {
                    Ok(messages) => {
                        for msg in messages {
                            let json = serde_json::to_string_pretty(&msg)?;
                            println!("{}", json);
                        }
                    }
                    Err(e) => {
                        print_error(&format!("Receive error: {}", e));
                        break;
                    }
                }
                std::thread::sleep(poll_interval);
            }
        }

        ChannelType::Thread => {
            print_error("Thread channels cannot be listened via CLI (they are in-process only)");
            return Err("Thread channels are in-process only".into());
        }
    }

    Ok(())
}
