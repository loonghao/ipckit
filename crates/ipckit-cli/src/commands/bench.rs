//! Benchmark command implementation

use super::{channel_type_name, print_info};
use crate::{ChannelType, OutputFormat};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::{Duration, Instant};

pub fn bench(
    channel_type: ChannelType,
    iterations: u64,
    message_size: usize,
    warmup: u64,
    format: OutputFormat,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    print_info(&format!(
        "Benchmarking {} with {} iterations, {} byte messages",
        channel_type_name(channel_type),
        iterations,
        message_size
    ));

    // Create test message
    let message: Vec<u8> = (0..message_size).map(|i| (i % 256) as u8).collect();

    // Run benchmark based on channel type
    let results = match channel_type {
        ChannelType::Thread => bench_thread_channel(&message, iterations, warmup, verbose)?,
        ChannelType::Pipe => {
            print_info("Note: Pipe benchmark requires separate server/client processes");
            print_info("Using in-memory simulation for demonstration");
            bench_simulated(&message, iterations, warmup, verbose)?
        }
        ChannelType::Socket => {
            print_info("Note: Socket benchmark requires separate server/client processes");
            print_info("Using in-memory simulation for demonstration");
            bench_simulated(&message, iterations, warmup, verbose)?
        }
        ChannelType::Shm => bench_shared_memory(&message, iterations, warmup, verbose)?,
        ChannelType::File => {
            print_info("Note: File channel benchmark uses disk I/O");
            bench_file_channel(&message, iterations, warmup, verbose)?
        }
    };

    // Print results
    print_results(&results, format);

    Ok(())
}

#[derive(Debug)]
struct BenchResults {
    channel_type: String,
    iterations: u64,
    message_size: usize,
    total_time: Duration,
    throughput_msgs: f64,
    throughput_bytes: f64,
    avg_latency: Duration,
    min_latency: Duration,
    max_latency: Duration,
    p50_latency: Duration,
    p95_latency: Duration,
    p99_latency: Duration,
}

fn bench_thread_channel(
    message: &[u8],
    iterations: u64,
    warmup: u64,
    verbose: bool,
) -> Result<BenchResults, Box<dyn std::error::Error>> {
    use ipckit::ThreadChannel;

    let (tx, rx) = ThreadChannel::<Vec<u8>>::unbounded();
    let message = message.to_vec();
    let msg_clone = message.clone();

    // Spawn receiver thread
    let receiver = std::thread::spawn(move || {
        for _ in 0..(warmup + iterations) {
            let _ = rx.recv();
        }
    });

    // Warmup
    if verbose && warmup > 0 {
        print_info(&format!("Warming up with {} iterations...", warmup));
    }
    for _ in 0..warmup {
        tx.send(msg_clone.clone()).ok();
    }

    // Benchmark
    let pb = ProgressBar::new(iterations);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut latencies = Vec::with_capacity(iterations as usize);
    let start = Instant::now();

    for _ in 0..iterations {
        let iter_start = Instant::now();
        tx.send(msg_clone.clone()).ok();
        latencies.push(iter_start.elapsed());
        pb.inc(1);
    }

    let total_time = start.elapsed();
    pb.finish_with_message("Done");

    receiver.join().ok();

    Ok(calculate_results(
        "Thread Channel",
        message.len(),
        iterations,
        total_time,
        latencies,
    ))
}

fn bench_shared_memory(
    message: &[u8],
    iterations: u64,
    warmup: u64,
    verbose: bool,
) -> Result<BenchResults, Box<dyn std::error::Error>> {
    use ipckit::SharedMemory;

    let name = format!("ipckit_bench_{}", std::process::id());
    let mut shm = SharedMemory::create(&name, message.len())?;

    // Warmup
    if verbose && warmup > 0 {
        print_info(&format!("Warming up with {} iterations...", warmup));
    }
    for _ in 0..warmup {
        shm.write(0, message)?;
        let _ = shm.read(0, message.len())?;
    }

    // Benchmark
    let pb = ProgressBar::new(iterations);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut latencies = Vec::with_capacity(iterations as usize);
    let start = Instant::now();

    for _ in 0..iterations {
        let iter_start = Instant::now();
        shm.write(0, message)?;
        let _ = shm.read(0, message.len())?;
        latencies.push(iter_start.elapsed());
        pb.inc(1);
    }

    let total_time = start.elapsed();
    pb.finish_with_message("Done");

    Ok(calculate_results(
        "Shared Memory",
        message.len(),
        iterations,
        total_time,
        latencies,
    ))
}

fn bench_file_channel(
    message: &[u8],
    iterations: u64,
    warmup: u64,
    verbose: bool,
) -> Result<BenchResults, Box<dyn std::error::Error>> {
    use ipckit::FileChannel;

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("ipckit_bench_{}", std::process::id()));
    let channel = FileChannel::backend(path.to_str().unwrap())?;

    // Warmup
    if verbose && warmup > 0 {
        print_info(&format!("Warming up with {} iterations...", warmup));
    }
    for _ in 0..warmup {
        channel.send_event("bench", serde_json::json!({"data": "warmup"}))?;
    }

    // Benchmark
    let pb = ProgressBar::new(iterations);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut latencies = Vec::with_capacity(iterations as usize);
    let start = Instant::now();

    for i in 0..iterations {
        let iter_start = Instant::now();
        channel.send_event("bench", serde_json::json!({"iteration": i}))?;
        latencies.push(iter_start.elapsed());
        pb.inc(1);
    }

    let total_time = start.elapsed();
    pb.finish_with_message("Done");

    Ok(calculate_results(
        "File Channel",
        message.len(),
        iterations,
        total_time,
        latencies,
    ))
}

fn bench_simulated(
    message: &[u8],
    iterations: u64,
    warmup: u64,
    verbose: bool,
) -> Result<BenchResults, Box<dyn std::error::Error>> {
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let message = message.to_vec();
    let msg_clone = message.clone();

    // Spawn receiver thread
    let receiver = std::thread::spawn(move || {
        for _ in 0..(warmup + iterations) {
            let _ = rx.recv();
        }
    });

    // Warmup
    if verbose && warmup > 0 {
        print_info(&format!("Warming up with {} iterations...", warmup));
    }
    for _ in 0..warmup {
        tx.send(msg_clone.clone()).ok();
    }

    // Benchmark
    let pb = ProgressBar::new(iterations);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut latencies = Vec::with_capacity(iterations as usize);
    let start = Instant::now();

    for _ in 0..iterations {
        let iter_start = Instant::now();
        tx.send(msg_clone.clone()).ok();
        latencies.push(iter_start.elapsed());
        pb.inc(1);
    }

    let total_time = start.elapsed();
    pb.finish_with_message("Done");

    receiver.join().ok();

    Ok(calculate_results(
        "Simulated Channel",
        message.len(),
        iterations,
        total_time,
        latencies,
    ))
}

fn calculate_results(
    channel_type: &str,
    message_size: usize,
    iterations: u64,
    total_time: Duration,
    mut latencies: Vec<Duration>,
) -> BenchResults {
    latencies.sort();

    let throughput_msgs = iterations as f64 / total_time.as_secs_f64();
    let throughput_bytes = (iterations as f64 * message_size as f64) / total_time.as_secs_f64();

    let avg_latency = Duration::from_nanos(
        (latencies.iter().map(|d| d.as_nanos()).sum::<u128>() / latencies.len() as u128) as u64,
    );

    let min_latency = *latencies.first().unwrap_or(&Duration::ZERO);
    let max_latency = *latencies.last().unwrap_or(&Duration::ZERO);

    let p50_idx = (latencies.len() as f64 * 0.50) as usize;
    let p95_idx = (latencies.len() as f64 * 0.95) as usize;
    let p99_idx = (latencies.len() as f64 * 0.99) as usize;

    BenchResults {
        channel_type: channel_type.to_string(),
        iterations,
        message_size,
        total_time,
        throughput_msgs,
        throughput_bytes,
        avg_latency,
        min_latency,
        max_latency,
        p50_latency: latencies.get(p50_idx).copied().unwrap_or(Duration::ZERO),
        p95_latency: latencies.get(p95_idx).copied().unwrap_or(Duration::ZERO),
        p99_latency: latencies.get(p99_idx).copied().unwrap_or(Duration::ZERO),
    }
}

fn print_results(results: &BenchResults, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "channel_type": results.channel_type,
                "iterations": results.iterations,
                "message_size": results.message_size,
                "total_time_ms": results.total_time.as_millis(),
                "throughput_msgs_per_sec": results.throughput_msgs,
                "throughput_bytes_per_sec": results.throughput_bytes,
                "latency": {
                    "avg_us": results.avg_latency.as_micros(),
                    "min_us": results.min_latency.as_micros(),
                    "max_us": results.max_latency.as_micros(),
                    "p50_us": results.p50_latency.as_micros(),
                    "p95_us": results.p95_latency.as_micros(),
                    "p99_us": results.p99_latency.as_micros(),
                }
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        _ => {
            println!();
            println!("{}", style("Benchmark Results").bold().underlined());
            println!();
            println!("  Channel:        {}", style(&results.channel_type).cyan());
            println!("  Iterations:     {}", results.iterations);
            println!("  Message Size:   {} bytes", results.message_size);
            println!("  Total Time:     {:.3?}", results.total_time);
            println!();
            println!("{}", style("Throughput").bold());
            println!(
                "  Messages/sec:   {}",
                style(format!("{:.2}", results.throughput_msgs)).green()
            );
            println!(
                "  Bytes/sec:      {}",
                style(format_bytes(results.throughput_bytes)).green()
            );
            println!();
            println!("{}", style("Latency").bold());
            println!("  Average:        {:?}", results.avg_latency);
            println!("  Min:            {:?}", results.min_latency);
            println!("  Max:            {:?}", results.max_latency);
            println!("  p50:            {:?}", results.p50_latency);
            println!("  p95:            {:?}", results.p95_latency);
            println!("  p99:            {:?}", results.p99_latency);
            println!();
        }
    }
}

fn format_bytes(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes >= GB {
        format!("{:.2} GB/s", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB/s", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB/s", bytes / KB)
    } else {
        format!("{:.2} B/s", bytes)
    }
}
