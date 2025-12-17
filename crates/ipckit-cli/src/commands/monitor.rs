//! Channel monitoring command

use crate::{ChannelType, OutputFormat};
use console::{style, Term};
use serde::Serialize;
use std::io::Write;
use std::time::{Duration, Instant};

use super::{channel_type_name, print_info};

/// Monitor channel activity
pub fn monitor(
    channel_type: Option<ChannelType>,
    name: Option<String>,
    format: OutputFormat,
    interval_ms: u64,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if verbose {
        match (&channel_type, &name) {
            (Some(ct), Some(n)) => {
                print_info(&format!(
                    "Monitoring {} channel '{}'",
                    channel_type_name(*ct),
                    n
                ));
            }
            _ => {
                print_info("Monitoring all channels");
            }
        }
    }

    let interval = Duration::from_millis(interval_ms);

    match format {
        OutputFormat::Json => monitor_json(channel_type, name, interval),
        OutputFormat::Text | OutputFormat::Hex => monitor_text(channel_type, name, interval),
    }
}

fn monitor_json(
    _channel_type: Option<ChannelType>,
    _name: Option<String>,
    interval: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    loop {
        let stats = collect_stats();
        let output = serde_json::json!({
            "timestamp": chrono_now(),
            "uptime_secs": start.elapsed().as_secs(),
            "channels": stats,
        });

        println!("{}", serde_json::to_string(&output)?);

        std::thread::sleep(interval);
    }
}

fn monitor_text(
    _channel_type: Option<ChannelType>,
    _name: Option<String>,
    interval: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let term = Term::stdout();
    let start = Instant::now();

    loop {
        // Clear screen and move to top
        let _ = term.clear_screen();

        // Header
        let _ = writeln!(
            &term,
            "{}",
            style("╔══════════════════════════════════════════════════════════════╗")
                .cyan()
                .bold()
        );
        let _ = writeln!(
            &term,
            "{}",
            style("║                    ipckit Channel Monitor                    ║")
                .cyan()
                .bold()
        );
        let _ = writeln!(
            &term,
            "{}",
            style("╚══════════════════════════════════════════════════════════════╝")
                .cyan()
                .bold()
        );
        let _ = writeln!(&term);

        // Uptime
        let uptime = start.elapsed();
        let _ = writeln!(
            &term,
            "  {} {} | {} {}",
            style("Uptime:").dim(),
            style(format_duration(uptime)).green(),
            style("Updated:").dim(),
            style(chrono_now()).yellow()
        );
        let _ = writeln!(&term);

        // Stats table header
        let _ = writeln!(
            &term,
            "  {}",
            style("┌──────────────────┬──────────┬──────────┬──────────┬──────────┐").dim()
        );
        let _ = writeln!(
            &term,
            "  {} {:^16} {} {:^8} {} {:^8} {} {:^8} {} {:^8} {}",
            style("│").dim(),
            style("Channel").bold(),
            style("│").dim(),
            style("Sent").bold(),
            style("│").dim(),
            style("Recv").bold(),
            style("│").dim(),
            style("Errors").bold(),
            style("│").dim(),
            style("Latency").bold(),
            style("│").dim()
        );
        let _ = writeln!(
            &term,
            "  {}",
            style("├──────────────────┼──────────┼──────────┼──────────┼──────────┤").dim()
        );

        // Channel stats
        let stats = collect_stats();
        if stats.is_empty() {
            let _ = writeln!(
                &term,
                "  {} {:^16} {} {:^8} {} {:^8} {} {:^8} {} {:^8} {}",
                style("│").dim(),
                style("(no channels)").dim(),
                style("│").dim(),
                "-",
                style("│").dim(),
                "-",
                style("│").dim(),
                "-",
                style("│").dim(),
                "-",
                style("│").dim()
            );
        } else {
            for stat in &stats {
                let _ = writeln!(
                    &term,
                    "  {} {:^16} {} {:>8} {} {:>8} {} {:>8} {} {:>8} {}",
                    style("│").dim(),
                    truncate(&stat.name, 16),
                    style("│").dim(),
                    format_count(stat.messages_sent),
                    style("│").dim(),
                    format_count(stat.messages_received),
                    style("│").dim(),
                    if stat.errors > 0 {
                        style(format_count(stat.errors)).red().to_string()
                    } else {
                        style(format_count(stat.errors)).green().to_string()
                    },
                    style("│").dim(),
                    format_latency(stat.avg_latency_us),
                    style("│").dim()
                );
            }
        }

        let _ = writeln!(
            &term,
            "  {}",
            style("└──────────────────┴──────────┴──────────┴──────────┴──────────┘").dim()
        );

        // Summary
        let _ = writeln!(&term);
        let total_sent: u64 = stats.iter().map(|s| s.messages_sent).sum();
        let total_recv: u64 = stats.iter().map(|s| s.messages_received).sum();
        let total_errors: u64 = stats.iter().map(|s| s.errors).sum();

        let _ = writeln!(
            &term,
            "  {} {} sent, {} received, {} errors",
            style("Total:").bold(),
            style(format_count(total_sent)).green(),
            style(format_count(total_recv)).blue(),
            if total_errors > 0 {
                style(format_count(total_errors)).red().to_string()
            } else {
                style(format_count(total_errors)).dim().to_string()
            }
        );

        let _ = writeln!(&term);
        let _ = writeln!(&term, "  {}", style("Press Ctrl+C to exit").dim());

        std::thread::sleep(interval);
    }
}

#[derive(Debug, Serialize)]
struct ChannelStats {
    name: String,
    messages_sent: u64,
    messages_received: u64,
    errors: u64,
    avg_latency_us: u64,
}

fn collect_stats() -> Vec<ChannelStats> {
    // In a real implementation, this would query actual channel metrics
    // For now, return sample data to demonstrate the UI
    vec![
        ChannelStats {
            name: "example_pipe".to_string(),
            messages_sent: 1234,
            messages_received: 1230,
            errors: 4,
            avg_latency_us: 125,
        },
        ChannelStats {
            name: "data_channel".to_string(),
            messages_sent: 5678,
            messages_received: 5678,
            errors: 0,
            avg_latency_us: 89,
        },
    ]
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn format_count(n: u64) -> String {
    if n < 1000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}K", n as f64 / 1000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

fn format_latency(us: u64) -> String {
    if us < 1000 {
        format!("{}µs", us)
    } else if us < 1_000_000 {
        format!("{:.1}ms", us as f64 / 1000.0)
    } else {
        format!("{:.1}s", us as f64 / 1_000_000.0)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len - 1])
    }
}
