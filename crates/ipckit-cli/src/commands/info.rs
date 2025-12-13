//! Info command implementation

use super::{channel_type_name, print_info};
use crate::ChannelType;
use console::style;
use ipckit::{LocalSocketStream, NamedPipe, SharedMemory};

pub fn info(
    channel_type: ChannelType,
    name: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("{}", style("Channel Information").bold().underlined());
    println!();
    println!(
        "  Type:   {}",
        style(channel_type_name(channel_type)).cyan()
    );
    println!("  Name:   {}", name);

    match channel_type {
        ChannelType::Pipe => {
            // Try to connect to check if pipe exists
            match NamedPipe::connect(name) {
                Ok(_) => {
                    println!("  Status: {}", style("Available").green());
                }
                Err(e) => {
                    println!("  Status: {}", style("Not available").red());
                    if verbose {
                        println!("  Error:  {}", e);
                    }
                }
            }

            // Platform-specific path
            #[cfg(windows)]
            println!("  Path:   \\\\.\\pipe\\{}", name);
            #[cfg(unix)]
            println!("  Path:   /tmp/{}.pipe", name);
        }

        ChannelType::Socket => {
            // Try to connect to check if socket exists
            match LocalSocketStream::connect(name) {
                Ok(_) => {
                    println!("  Status: {}", style("Listening").green());
                }
                Err(e) => {
                    println!("  Status: {}", style("Not listening").red());
                    if verbose {
                        println!("  Error:  {}", e);
                    }
                }
            }

            // Platform-specific path
            #[cfg(windows)]
            println!("  Path:   \\\\.\\pipe\\{}", name);
            #[cfg(unix)]
            println!("  Path:   /tmp/{}.sock", name);
        }

        ChannelType::Shm => {
            match SharedMemory::open(name) {
                Ok(shm) => {
                    println!("  Status: {}", style("Exists").green());
                    println!("  Size:   {} bytes", shm.size());
                }
                Err(e) => {
                    println!("  Status: {}", style("Does not exist").red());
                    if verbose {
                        println!("  Error:  {}", e);
                    }
                }
            }

            // Platform-specific path
            #[cfg(windows)]
            println!("  Path:   Global\\{}", name);
            #[cfg(unix)]
            println!("  Path:   /dev/shm/{}", name);
        }

        ChannelType::File => {
            use std::path::Path;

            let path = Path::new(name);
            if path.exists() {
                println!("  Status: {}", style("Exists").green());

                // Check for channel files
                let backend_to_frontend = path.join("backend_to_frontend.json");
                let frontend_to_backend = path.join("frontend_to_backend.json");

                if backend_to_frontend.exists() {
                    println!("  B->F:   {}", style("Present").green());
                    if let Ok(meta) = std::fs::metadata(&backend_to_frontend) {
                        println!("          {} bytes", meta.len());
                    }
                } else {
                    println!("  B->F:   {}", style("Missing").yellow());
                }

                if frontend_to_backend.exists() {
                    println!("  F->B:   {}", style("Present").green());
                    if let Ok(meta) = std::fs::metadata(&frontend_to_backend) {
                        println!("          {} bytes", meta.len());
                    }
                } else {
                    println!("  F->B:   {}", style("Missing").yellow());
                }
            } else {
                println!("  Status: {}", style("Does not exist").red());
            }

            println!("  Path:   {}", path.display());
        }

        ChannelType::Thread => {
            print_info("Thread channels are in-process only and cannot be inspected via CLI");
        }
    }

    println!();

    Ok(())
}
