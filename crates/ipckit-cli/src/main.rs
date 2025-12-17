//! # ipckit CLI
//!
//! A command-line interface for testing and debugging IPC channels.
//!
//! ## Usage
//!
//! ```bash
//! # Create a named pipe
//! ipckit create --type pipe --name my_pipe
//!
//! # Listen for messages
//! ipckit listen --type pipe --name my_pipe
//!
//! # Send a message
//! ipckit send --type pipe --name my_pipe "Hello, World!"
//!
//! # Benchmark
//! ipckit bench --type pipe --iterations 1000
//!
//! # Generate code
//! ipckit generate client --type pipe --name my_pipe
//!
//! # Monitor channels
//! ipckit monitor
//! ```

mod commands;

use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

/// IPC toolkit for Rust applications
#[derive(Parser)]
#[command(name = "ipckit")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, env = "IPCKIT_CONFIG")]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new IPC channel
    Create {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,

        /// Size (for shared memory)
        #[arg(short, long, default_value = "4096")]
        size: usize,
    },

    /// Listen on a channel and print messages
    Listen {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,

        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Timeout in milliseconds (0 = no timeout)
        #[arg(long, default_value = "0")]
        timeout: u64,
    },

    /// Send a message to a channel
    Send {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,

        /// Message to send (use '-' for stdin)
        message: String,

        /// Read message from file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Benchmark channel performance
    Bench {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Number of iterations
        #[arg(long, default_value = "1000")]
        iterations: u64,

        /// Message size in bytes
        #[arg(long, default_value = "1024")]
        message_size: usize,

        /// Number of warmup iterations
        #[arg(long, default_value = "100")]
        warmup: u64,

        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Show channel information
    Info {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,
    },

    /// Start an API server
    Serve {
        /// Socket path
        #[arg(short, long)]
        socket: Option<String>,

        /// Port for HTTP server (if using TCP)
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Generate code templates
    Generate {
        /// What to generate
        #[command(subcommand)]
        target: GenerateCommand,
    },

    /// Monitor channel activity
    Monitor {
        /// Channel type to monitor (optional, monitors all if not specified)
        #[arg(short = 't', long, value_enum)]
        channel_type: Option<ChannelType>,

        /// Channel name to monitor (optional)
        #[arg(short, long)]
        name: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Refresh interval in milliseconds
        #[arg(long, default_value = "1000")]
        interval: u64,
    },
}

#[derive(Subcommand, Clone)]
enum GenerateCommand {
    /// Generate client code
    Client {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,

        /// Output file (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate server code
    Server {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,

        /// Output file (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate Python bindings example
    Python {
        /// Channel type
        #[arg(short = 't', long, value_enum)]
        channel_type: ChannelType,

        /// Channel name
        #[arg(short, long)]
        name: String,

        /// Output file (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate IPC handler template
    Handler {
        /// Handler name
        #[arg(short, long)]
        name: String,

        /// Output file (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ChannelType {
    /// Named pipe
    Pipe,
    /// Shared memory
    Shm,
    /// Local socket
    Socket,
    /// File channel
    File,
    /// Thread channel
    Thread,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Plain text
    Text,
    /// JSON
    Json,
    /// Hex dump
    Hex,
}

#[derive(Clone, Copy, Debug)]
pub enum GenerateTarget {
    Client,
    Server,
    Python,
    Handler,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Create {
            channel_type,
            name,
            size,
        } => commands::create(channel_type, &name, size, cli.verbose),

        Commands::Listen {
            channel_type,
            name,
            format,
            timeout,
        } => commands::listen(channel_type, &name, format, timeout, cli.verbose),

        Commands::Send {
            channel_type,
            name,
            message,
            file,
        } => commands::send(channel_type, &name, &message, file, cli.verbose),

        Commands::Bench {
            channel_type,
            iterations,
            message_size,
            warmup,
            format,
        } => commands::bench(
            channel_type,
            iterations,
            message_size,
            warmup,
            format,
            cli.verbose,
        ),

        Commands::Completions { shell } => {
            commands::completions(shell);
            Ok(())
        }

        Commands::Info { channel_type, name } => commands::info(channel_type, &name, cli.verbose),

        Commands::Serve { socket, port } => commands::serve(socket, port, cli.verbose),

        Commands::Generate { target } => match target {
            GenerateCommand::Client {
                channel_type,
                name,
                output,
            } => commands::generate(
                GenerateTarget::Client,
                channel_type,
                &name,
                output,
                cli.verbose,
            ),
            GenerateCommand::Server {
                channel_type,
                name,
                output,
            } => commands::generate(
                GenerateTarget::Server,
                channel_type,
                &name,
                output,
                cli.verbose,
            ),
            GenerateCommand::Python {
                channel_type,
                name,
                output,
            } => commands::generate(
                GenerateTarget::Python,
                channel_type,
                &name,
                output,
                cli.verbose,
            ),
            GenerateCommand::Handler { name, output } => commands::generate(
                GenerateTarget::Handler,
                ChannelType::Pipe, // Default, not used for handler
                &name,
                output,
                cli.verbose,
            ),
        },

        Commands::Monitor {
            channel_type,
            name,
            format,
            interval,
        } => commands::monitor(channel_type, name, format, interval, cli.verbose),
    }
}
