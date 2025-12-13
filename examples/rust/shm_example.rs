//! Example: Using Shared Memory for IPC in Rust
//!
//! Run with: cargo run --example shm_example

use ipckit::{SharedMemory, Result};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let shm_name = format!("example_shm_{}", std::process::id());

    // Create shared memory
    println!("[Main] Creating shared memory: {}", shm_name);
    let mut shm = SharedMemory::create(&shm_name, 1024)?;
    println!("[Main] Size: {} bytes", shm.size());

    // Write some data
    let data = b"Hello, Shared Memory!";
    shm.write(0, data)?;
    println!("[Main] Wrote: {}", String::from_utf8_lossy(data));

    // Spawn reader thread (simulating another process)
    let reader_shm_name = shm_name.clone();
    let reader_handle = thread::spawn(move || -> Result<()> {
        thread::sleep(Duration::from_millis(100));

        println!("[Reader] Opening shared memory: {}", reader_shm_name);
        let shm = SharedMemory::open(&reader_shm_name)?;
        println!("[Reader] Size: {} bytes", shm.size());
        println!("[Reader] Is owner: {}", shm.is_owner());

        // Read data
        let data = shm.read(0, 21)?;
        println!("[Reader] Read: {}", String::from_utf8_lossy(&data));

        Ok(())
    });

    reader_handle.join().unwrap()?;

    // Write more data
    let new_data = b"Updated content!";
    shm.write(0, new_data)?;
    println!("[Main] Updated: {}", String::from_utf8_lossy(new_data));

    // Read back
    let read_data = shm.read(0, new_data.len())?;
    println!("[Main] Verified: {}", String::from_utf8_lossy(&read_data));

    println!("\nDone!");

    Ok(())
}
