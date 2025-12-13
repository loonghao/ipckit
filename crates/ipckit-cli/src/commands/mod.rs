//! CLI command implementations

mod bench;
mod completions;
mod create;
mod info;
mod listen;
mod send;
mod serve;

pub use bench::bench;
pub use completions::completions;
pub use create::create;
pub use info::info;
pub use listen::listen;
pub use send::send;
pub use serve::serve;

use crate::{ChannelType, OutputFormat};
use console::{style, Term};
use std::io::Write;

/// Print a success message
pub fn print_success(msg: &str) {
    let term = Term::stdout();
    let _ = writeln!(&term, "{} {}", style("✓").green().bold(), msg);
}

/// Print an info message
pub fn print_info(msg: &str) {
    let term = Term::stdout();
    let _ = writeln!(&term, "{} {}", style("ℹ").blue().bold(), msg);
}

/// Print a warning message
#[allow(dead_code)]
pub fn print_warning(msg: &str) {
    let term = Term::stderr();
    let _ = writeln!(&term, "{} {}", style("⚠").yellow().bold(), msg);
}

/// Print an error message
pub fn print_error(msg: &str) {
    let term = Term::stderr();
    let _ = writeln!(&term, "{} {}", style("✗").red().bold(), msg);
}

/// Format bytes as hex dump
pub fn hex_dump(data: &[u8]) -> String {
    let mut output = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        // Offset
        output.push_str(&format!("{:08x}  ", i * 16));

        // Hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            output.push_str(&format!("{:02x} ", byte));
            if j == 7 {
                output.push(' ');
            }
        }

        // Padding
        if chunk.len() < 16 {
            for j in chunk.len()..16 {
                output.push_str("   ");
                if j == 7 {
                    output.push(' ');
                }
            }
        }

        output.push_str(" |");

        // ASCII
        for byte in chunk {
            if *byte >= 0x20 && *byte < 0x7f {
                output.push(*byte as char);
            } else {
                output.push('.');
            }
        }

        output.push_str("|\n");
    }
    output
}

/// Format data according to output format
pub fn format_output(data: &[u8], format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => String::from_utf8_lossy(data).to_string(),
        OutputFormat::Json => {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| hex_dump(data))
            } else {
                // Try to wrap as string
                serde_json::json!({
                    "data": String::from_utf8_lossy(data),
                    "length": data.len()
                })
                .to_string()
            }
        }
        OutputFormat::Hex => hex_dump(data),
    }
}

/// Get channel type display name
pub fn channel_type_name(ct: ChannelType) -> &'static str {
    match ct {
        ChannelType::Pipe => "Named Pipe",
        ChannelType::Shm => "Shared Memory",
        ChannelType::Socket => "Local Socket",
        ChannelType::File => "File Channel",
        ChannelType::Thread => "Thread Channel",
    }
}
