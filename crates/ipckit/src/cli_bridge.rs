//! # CLI Bridge
//!
//! This module provides a CLI integration bridge that allows any CLI tool to easily
//! integrate into the ipckit ecosystem, enabling real-time communication with frontends.
//! Similar to how Docker CLI integrates with Docker Desktop.
//!
//! ## Features
//!
//! - Minimal invasiveness - existing CLI only needs minimal modifications
//! - Automatic output capture (stdout/stderr)
//! - Progress bar parsing
//! - Bidirectional communication (CLI can send events, frontend can send commands)
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit::{CliBridge, WrappedCommand};
//!
//! // Method 1: Direct bridge usage
//! let bridge = CliBridge::connect()?;
//! bridge.register_task("My CLI Task", "build")?;
//!
//! bridge.log("info", "Starting build...");
//!
//! for i in 0..100 {
//!     if bridge.is_cancelled() {
//!         bridge.fail("Cancelled by user");
//!         return Ok(());
//!     }
//!     bridge.set_progress(i + 1, Some(&format!("Step {}/100", i + 1)));
//! }
//!
//! bridge.complete(json!({"success": true}));
//!
//! // Method 2: Wrap existing command
//! let output = WrappedCommand::new("cargo")
//!     .args(["build", "--release"])
//!     .task("Build Project", "build")
//!     .progress_parser(parsers::PercentageParser)
//!     .run()?;
//! ```

use crate::api_server::ApiClient;
use crate::error::{IpcError, Result};
use crate::socket_server::SocketServerConfig;
use crate::task_manager::CancellationToken;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// CLI bridge configuration.
#[derive(Clone)]
pub struct CliBridgeConfig {
    /// API server address (socket path)
    pub server_url: String,
    /// Auto-register as task when connecting
    pub auto_register: bool,
    /// Capture stdout
    pub capture_stdout: bool,
    /// Capture stderr
    pub capture_stderr: bool,
    /// Progress parser (optional)
    pub progress_parser: Option<Arc<dyn ProgressParser>>,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Retry count for connection
    pub retry_count: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl std::fmt::Debug for CliBridgeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliBridgeConfig")
            .field("server_url", &self.server_url)
            .field("auto_register", &self.auto_register)
            .field("capture_stdout", &self.capture_stdout)
            .field("capture_stderr", &self.capture_stderr)
            .field("progress_parser", &self.progress_parser.is_some())
            .field("connect_timeout", &self.connect_timeout)
            .field("retry_count", &self.retry_count)
            .field("retry_delay", &self.retry_delay)
            .finish()
    }
}

impl Default for CliBridgeConfig {
    fn default() -> Self {
        Self {
            server_url: SocketServerConfig::default().path,
            auto_register: true,
            capture_stdout: true,
            capture_stderr: true,
            progress_parser: None,
            connect_timeout: Duration::from_secs(5),
            retry_count: 3,
            retry_delay: Duration::from_millis(500),
        }
    }
}

impl CliBridgeConfig {
    /// Create a new configuration with the specified server URL.
    pub fn with_server(url: &str) -> Self {
        Self {
            server_url: url.to_string(),
            ..Default::default()
        }
    }

    /// Set the progress parser.
    pub fn progress_parser<P: ProgressParser + 'static>(mut self, parser: P) -> Self {
        self.progress_parser = Some(Arc::new(parser));
        self
    }

    /// Disable auto-registration.
    pub fn no_auto_register(mut self) -> Self {
        self.auto_register = false;
        self
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("IPCKIT_SERVER_URL") {
            config.server_url = url;
        }

        if let Ok(auto_reg) = std::env::var("IPCKIT_AUTO_REGISTER") {
            config.auto_register = auto_reg.to_lowercase() != "false";
        }

        config
    }
}

/// Progress information parsed from output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Current progress value
    pub current: u64,
    /// Total value (for percentage calculation)
    pub total: u64,
    /// Optional message
    pub message: Option<String>,
}

impl ProgressInfo {
    /// Create a new progress info.
    pub fn new(current: u64, total: u64) -> Self {
        Self {
            current,
            total,
            message: None,
        }
    }

    /// Create progress info with a message.
    pub fn with_message(current: u64, total: u64, message: &str) -> Self {
        Self {
            current,
            total,
            message: Some(message.to_string()),
        }
    }

    /// Get the percentage (0-100).
    pub fn percentage(&self) -> u8 {
        if self.total == 0 {
            0
        } else {
            ((self.current * 100) / self.total).min(100) as u8
        }
    }
}

/// Trait for parsing progress from output lines.
pub trait ProgressParser: Send + Sync {
    /// Parse progress from an output line.
    fn parse(&self, line: &str) -> Option<ProgressInfo>;
}

/// Built-in progress parsers.
pub mod parsers {
    use super::*;
    use regex::Regex;
    use std::sync::LazyLock;

    /// Percentage parser - matches patterns like "50%", "Progress: 50%", etc.
    #[derive(Debug, Clone, Default)]
    pub struct PercentageParser;

    impl ProgressParser for PercentageParser {
        fn parse(&self, line: &str) -> Option<ProgressInfo> {
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"(\d{1,3})%").expect("Invalid regex"));

            RE.captures(line).and_then(|caps| {
                caps.get(1)
                    .and_then(|m| m.as_str().parse::<u64>().ok())
                    .map(|pct| ProgressInfo::new(pct.min(100), 100))
            })
        }
    }

    /// Fraction parser - matches patterns like "5/10", "[5/10]", etc.
    #[derive(Debug, Clone, Default)]
    pub struct FractionParser;

    impl ProgressParser for FractionParser {
        fn parse(&self, line: &str) -> Option<ProgressInfo> {
            static RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"(\d+)\s*/\s*(\d+)").expect("Invalid regex"));

            RE.captures(line).and_then(|caps| {
                let current = caps.get(1)?.as_str().parse::<u64>().ok()?;
                let total = caps.get(2)?.as_str().parse::<u64>().ok()?;
                Some(ProgressInfo::new(current, total))
            })
        }
    }

    /// Progress bar parser - matches patterns like "[=====>    ] 50%"
    #[derive(Debug, Clone, Default)]
    pub struct ProgressBarParser;

    impl ProgressParser for ProgressBarParser {
        fn parse(&self, line: &str) -> Option<ProgressInfo> {
            static RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"\[([=\-#>]+)\s*\]\s*(\d{1,3})%").expect("Invalid regex")
            });

            RE.captures(line).and_then(|caps| {
                caps.get(2)
                    .and_then(|m| m.as_str().parse::<u64>().ok())
                    .map(|pct| ProgressInfo::new(pct.min(100), 100))
            })
        }
    }

    /// Composite parser - tries multiple parsers in order.
    #[derive(Default)]
    pub struct CompositeParser {
        parsers: Vec<Arc<dyn ProgressParser>>,
    }

    impl CompositeParser {
        /// Create a new composite parser.
        pub fn new() -> Self {
            Self::default()
        }

        /// Add a parser.
        #[allow(clippy::should_implement_trait)]
        pub fn add<P: ProgressParser + 'static>(mut self, parser: P) -> Self {
            self.parsers.push(Arc::new(parser));
            self
        }

        /// Create a default composite parser with all built-in parsers.
        pub fn default_all() -> Self {
            Self::new()
                .add(PercentageParser)
                .add(FractionParser)
                .add(ProgressBarParser)
        }
    }

    impl ProgressParser for CompositeParser {
        fn parse(&self, line: &str) -> Option<ProgressInfo> {
            for parser in &self.parsers {
                if let Some(info) = parser.parse(line) {
                    return Some(info);
                }
            }
            None
        }
    }
}

/// Internal state for the CLI bridge.
struct BridgeState {
    task_id: Option<String>,
    task_name: Option<String>,
    task_type: Option<String>,
    progress: u8,
    progress_message: Option<String>,
    cancelled: AtomicBool,
    completed: AtomicBool,
}

impl Default for BridgeState {
    fn default() -> Self {
        Self {
            task_id: None,
            task_name: None,
            task_type: None,
            progress: 0,
            progress_message: None,
            cancelled: AtomicBool::new(false),
            completed: AtomicBool::new(false),
        }
    }
}

/// CLI Bridge for integrating CLI tools with ipckit.
pub struct CliBridge {
    config: CliBridgeConfig,
    client: Option<ApiClient>,
    state: Arc<RwLock<BridgeState>>,
    cancel_token: CancellationToken,
}

impl CliBridge {
    /// Create a new CLI bridge with the given configuration.
    pub fn new(config: CliBridgeConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: None,
            state: Arc::new(RwLock::new(BridgeState::default())),
            cancel_token: CancellationToken::new(),
        })
    }

    /// Connect with default configuration.
    pub fn connect() -> Result<Self> {
        Self::connect_with_config(CliBridgeConfig::from_env())
    }

    /// Connect with the given configuration.
    pub fn connect_with_config(config: CliBridgeConfig) -> Result<Self> {
        let client = ApiClient::new(&config.server_url);

        Ok(Self {
            config,
            client: Some(client),
            state: Arc::new(RwLock::new(BridgeState::default())),
            cancel_token: CancellationToken::new(),
        })
    }

    /// Register the current process as a task.
    pub fn register_task(&self, name: &str, task_type: &str) -> Result<String> {
        let task_id = format!(
            "cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        {
            let mut state = self.state.write();
            state.task_id = Some(task_id.clone());
            state.task_name = Some(name.to_string());
            state.task_type = Some(task_type.to_string());
        }

        // If connected, register with the server
        if let Some(ref client) = self.client {
            let _ = client.post(
                "/v1/tasks",
                Some(serde_json::json!({
                    "id": task_id,
                    "name": name,
                    "type": task_type,
                    "status": "running"
                })),
            );
        }

        Ok(task_id)
    }

    /// Get the current task ID.
    pub fn task_id(&self) -> Option<String> {
        self.state.read().task_id.clone()
    }

    /// Set the progress.
    pub fn set_progress(&self, progress: u8, message: Option<&str>) {
        let progress = progress.min(100);

        {
            let mut state = self.state.write();
            state.progress = progress;
            if let Some(msg) = message {
                state.progress_message = Some(msg.to_string());
            }
        }

        // Send to server if connected
        if let (Some(ref client), Some(task_id)) = (&self.client, self.task_id()) {
            let _ = client.post(
                &format!("/v1/tasks/{}/progress", task_id),
                Some(serde_json::json!({
                    "progress": progress,
                    "message": message
                })),
            );
        }
    }

    /// Log a message.
    pub fn log(&self, level: &str, message: &str) {
        // Print to stderr for CLI visibility
        eprintln!("[{}] {}", level.to_uppercase(), message);

        // Send to server if connected
        if let (Some(ref client), Some(task_id)) = (&self.client, self.task_id()) {
            let _ = client.post(
                &format!("/v1/tasks/{}/logs", task_id),
                Some(serde_json::json!({
                    "level": level,
                    "message": message
                })),
            );
        }
    }

    /// Send stdout line.
    pub fn stdout(&self, line: &str) {
        println!("{}", line);

        if let (Some(ref client), Some(task_id)) = (&self.client, self.task_id()) {
            let _ = client.post(
                &format!("/v1/tasks/{}/stdout", task_id),
                Some(serde_json::json!({ "line": line })),
            );
        }
    }

    /// Send stderr line.
    pub fn stderr(&self, line: &str) {
        eprintln!("{}", line);

        if let (Some(ref client), Some(task_id)) = (&self.client, self.task_id()) {
            let _ = client.post(
                &format!("/v1/tasks/{}/stderr", task_id),
                Some(serde_json::json!({ "line": line })),
            );
        }
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled() || self.state.read().cancelled.load(Ordering::SeqCst)
    }

    /// Get the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// Mark the task as complete.
    pub fn complete(&self, result: serde_json::Value) {
        self.state.write().completed.store(true, Ordering::SeqCst);

        if let (Some(ref client), Some(task_id)) = (&self.client, self.task_id()) {
            let _ = client.post(
                &format!("/v1/tasks/{}/complete", task_id),
                Some(serde_json::json!({ "result": result })),
            );
        }
    }

    /// Mark the task as failed.
    pub fn fail(&self, error: &str) {
        self.state.write().completed.store(true, Ordering::SeqCst);

        if let (Some(ref client), Some(task_id)) = (&self.client, self.task_id()) {
            let _ = client.post(
                &format!("/v1/tasks/{}/fail", task_id),
                Some(serde_json::json!({ "error": error })),
            );
        }
    }

    /// Create a stdout wrapper that auto-forwards output.
    pub fn wrap_stdout(&self) -> WrappedWriter {
        WrappedWriter::new(
            self.config.server_url.clone(),
            self.task_id(),
            OutputType::Stdout,
            self.config.progress_parser.clone(),
            Arc::clone(&self.state),
        )
    }

    /// Create a stderr wrapper that auto-forwards output.
    pub fn wrap_stderr(&self) -> WrappedWriter {
        WrappedWriter::new(
            self.config.server_url.clone(),
            self.task_id(),
            OutputType::Stderr,
            None,
            Arc::clone(&self.state),
        )
    }
}

/// Output type for wrapped writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    Stdout,
    Stderr,
}

/// A writer that wraps stdout/stderr and forwards to the server.
pub struct WrappedWriter {
    client: Option<ApiClient>,
    task_id: Option<String>,
    output_type: OutputType,
    progress_parser: Option<Arc<dyn ProgressParser>>,
    state: Arc<RwLock<BridgeState>>,
    buffer: Vec<u8>,
}

impl WrappedWriter {
    fn new(
        server_url: String,
        task_id: Option<String>,
        output_type: OutputType,
        progress_parser: Option<Arc<dyn ProgressParser>>,
        state: Arc<RwLock<BridgeState>>,
    ) -> Self {
        let client = Some(ApiClient::new(&server_url));
        Self {
            client,
            task_id,
            output_type,
            progress_parser,
            state,
            buffer: Vec::new(),
        }
    }

    fn process_line(&mut self, line: &str) {
        // Check for progress
        if let Some(ref parser) = self.progress_parser {
            if let Some(info) = parser.parse(line) {
                let mut state = self.state.write();
                state.progress = info.percentage();
                state.progress_message = info.message.clone();
            }
        }

        // Send to server
        if let (Some(ref client), Some(ref task_id)) = (&self.client, &self.task_id) {
            let endpoint = match self.output_type {
                OutputType::Stdout => format!("/v1/tasks/{}/stdout", task_id),
                OutputType::Stderr => format!("/v1/tasks/{}/stderr", task_id),
            };
            let _ = client.post(&endpoint, Some(serde_json::json!({ "line": line })));
        }
    }
}

impl Write for WrappedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // Also write to actual stdout/stderr
        let written = match self.output_type {
            OutputType::Stdout => std::io::stdout().write(buf)?,
            OutputType::Stderr => std::io::stderr().write(buf)?,
        };

        // Buffer and process lines
        self.buffer.extend_from_slice(&buf[..written]);

        // Process complete lines
        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(&self.buffer[..pos]).to_string();
            self.buffer.drain(..=pos);
            self.process_line(&line);
        }

        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // Process any remaining buffer
        if !self.buffer.is_empty() {
            let line = String::from_utf8_lossy(&self.buffer).to_string();
            self.buffer.clear();
            self.process_line(&line);
        }

        match self.output_type {
            OutputType::Stdout => std::io::stdout().flush(),
            OutputType::Stderr => std::io::stderr().flush(),
        }
    }
}

/// Output from a wrapped command.
#[derive(Debug)]
pub struct CommandOutput {
    /// Exit code
    pub exit_code: i32,
    /// Captured stdout
    pub stdout: String,
    /// Captured stderr
    pub stderr: String,
    /// Duration of execution
    pub duration: Duration,
}

/// A wrapped command that integrates with the CLI bridge.
pub struct WrappedCommand {
    command: Command,
    task_name: String,
    task_type: String,
    progress_parser: Option<Arc<dyn ProgressParser>>,
    bridge_config: CliBridgeConfig,
}

impl WrappedCommand {
    /// Create a new wrapped command.
    pub fn new(program: &str) -> Self {
        let mut command = Command::new(program);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        Self {
            command,
            task_name: program.to_string(),
            task_type: "command".to_string(),
            progress_parser: None,
            bridge_config: CliBridgeConfig::from_env(),
        }
    }

    /// Set the task info.
    pub fn task(mut self, name: &str, task_type: &str) -> Self {
        self.task_name = name.to_string();
        self.task_type = task_type.to_string();
        self
    }

    /// Add an argument.
    pub fn arg(mut self, arg: &str) -> Self {
        self.command.arg(arg);
        self
    }

    /// Add multiple arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.command.args(args);
        self
    }

    /// Set the working directory.
    pub fn current_dir(mut self, dir: &std::path::Path) -> Self {
        self.command.current_dir(dir);
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.command.env(key, value);
        self
    }

    /// Set the progress parser.
    pub fn progress_parser<P: ProgressParser + 'static>(mut self, parser: P) -> Self {
        self.progress_parser = Some(Arc::new(parser));
        self
    }

    /// Set the bridge configuration.
    pub fn bridge_config(mut self, config: CliBridgeConfig) -> Self {
        self.bridge_config = config;
        self
    }

    /// Execute the command (blocking).
    pub fn run(mut self) -> Result<CommandOutput> {
        let start = Instant::now();

        // Try to connect to bridge
        let bridge = CliBridge::connect_with_config(self.bridge_config.clone()).ok();

        // Register task if connected
        if let Some(ref bridge) = bridge {
            let _ = bridge.register_task(&self.task_name, &self.task_type);
        }

        // Spawn the command
        let mut child = self.command.spawn().map_err(IpcError::Io)?;

        // Capture output
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let progress_parser = self.progress_parser.clone();
        let bridge_clone = bridge.as_ref().map(|b| b.state.clone());

        // Spawn stdout reader
        let stdout_handle: Option<JoinHandle<String>> = stdout.map(|out| {
            let parser = progress_parser.clone();
            let state = bridge_clone.clone();
            thread::spawn(move || {
                let mut output = String::new();
                let reader = BufReader::new(out);
                for line_result in reader.lines() {
                    let Ok(line) = line_result else { break };
                    println!("{}", line);
                    output.push_str(&line);
                    output.push('\n');

                    // Parse progress
                    if let (Some(ref parser), Some(ref state)) = (&parser, &state) {
                        if let Some(info) = parser.parse(&line) {
                            let mut s = state.write();
                            s.progress = info.percentage();
                            s.progress_message = info.message;
                        }
                    }
                }
                output
            })
        });

        // Spawn stderr reader
        let stderr_handle: Option<JoinHandle<String>> = stderr.map(|err| {
            thread::spawn(move || {
                let mut output = String::new();
                let reader = BufReader::new(err);
                for line_result in reader.lines() {
                    let Ok(line) = line_result else { break };
                    eprintln!("{}", line);
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            })
        });

        // Wait for command to complete
        let status = child.wait().map_err(IpcError::Io)?;

        // Collect output
        let stdout_output = stdout_handle
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default();
        let stderr_output = stderr_handle
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default();

        let duration = start.elapsed();
        let exit_code = status.code().unwrap_or(-1);

        // Report completion
        if let Some(ref bridge) = bridge {
            if exit_code == 0 {
                bridge.complete(serde_json::json!({
                    "exit_code": exit_code,
                    "duration_ms": duration.as_millis()
                }));
            } else {
                bridge.fail(&format!("Command exited with code {}", exit_code));
            }
        }

        Ok(CommandOutput {
            exit_code,
            stdout: stdout_output,
            stderr: stderr_output,
            duration,
        })
    }

    /// Execute the command (non-blocking).
    pub fn spawn(mut self) -> Result<WrappedChild> {
        // Try to connect to bridge
        let bridge = CliBridge::connect_with_config(self.bridge_config.clone()).ok();

        // Register task if connected
        let task_id = if let Some(ref bridge) = bridge {
            bridge.register_task(&self.task_name, &self.task_type).ok()
        } else {
            None
        };

        // Spawn the command
        let child = self.command.spawn().map_err(IpcError::Io)?;

        Ok(WrappedChild {
            child,
            bridge,
            task_id,
            start_time: Instant::now(),
        })
    }
}

/// A wrapped child process.
pub struct WrappedChild {
    child: Child,
    bridge: Option<CliBridge>,
    task_id: Option<String>,
    start_time: Instant,
}

impl WrappedChild {
    /// Wait for the process to complete.
    pub fn wait(mut self) -> Result<CommandOutput> {
        let status = self.child.wait().map_err(IpcError::Io)?;
        let duration = self.start_time.elapsed();
        let exit_code = status.code().unwrap_or(-1);

        // Report completion
        if let Some(ref bridge) = self.bridge {
            if exit_code == 0 {
                bridge.complete(serde_json::json!({
                    "exit_code": exit_code,
                    "duration_ms": duration.as_millis()
                }));
            } else {
                bridge.fail(&format!("Command exited with code {}", exit_code));
            }
        }

        Ok(CommandOutput {
            exit_code,
            stdout: String::new(), // Not captured in spawn mode
            stderr: String::new(),
            duration,
        })
    }

    /// Send a cancel signal to the process.
    pub fn cancel(&mut self) -> Result<()> {
        self.child.kill().map_err(IpcError::Io)
    }

    /// Get the task ID.
    pub fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }

    /// Check if the process has exited.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.child.try_wait().map_err(IpcError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ProgressParser Tests ====================

    #[test]
    fn test_percentage_parser() {
        let parser = parsers::PercentageParser;

        assert_eq!(parser.parse("50%").map(|p| p.percentage()), Some(50));
        assert_eq!(
            parser.parse("Progress: 75%").map(|p| p.percentage()),
            Some(75)
        );
        assert_eq!(
            parser.parse("Downloading... 100%").map(|p| p.percentage()),
            Some(100)
        );
        assert_eq!(
            parser.parse("No progress here").map(|p| p.percentage()),
            None
        );
    }

    #[test]
    fn test_percentage_parser_edge_cases() {
        let parser = parsers::PercentageParser;

        // Edge cases
        assert_eq!(parser.parse("0%").map(|p| p.percentage()), Some(0));
        assert_eq!(parser.parse("1%").map(|p| p.percentage()), Some(1));
        assert_eq!(parser.parse("99%").map(|p| p.percentage()), Some(99));

        // Values over 100 should be capped
        assert_eq!(parser.parse("150%").map(|p| p.percentage()), Some(100));

        // Multiple percentages - should match first
        let info = parser.parse("Step 1: 25% complete, overall: 50%");
        assert_eq!(info.map(|p| p.percentage()), Some(25));
    }

    #[test]
    fn test_fraction_parser() {
        let parser = parsers::FractionParser;

        let info = parser.parse("5/10").unwrap();
        assert_eq!(info.current, 5);
        assert_eq!(info.total, 10);
        assert_eq!(info.percentage(), 50);

        let info = parser.parse("[3/4] Installing...").unwrap();
        assert_eq!(info.current, 3);
        assert_eq!(info.total, 4);
        assert_eq!(info.percentage(), 75);

        assert!(parser.parse("No fraction").is_none());
    }

    #[test]
    fn test_fraction_parser_edge_cases() {
        let parser = parsers::FractionParser;

        // Zero cases
        let info = parser.parse("0/10").unwrap();
        assert_eq!(info.percentage(), 0);

        let info = parser.parse("10/10").unwrap();
        assert_eq!(info.percentage(), 100);

        // Division by zero protection
        let info = parser.parse("5/0").unwrap();
        assert_eq!(info.percentage(), 0);

        // Spaces around slash
        let info = parser.parse("3 / 5").unwrap();
        assert_eq!(info.current, 3);
        assert_eq!(info.total, 5);

        // Large numbers
        let info = parser.parse("999/1000").unwrap();
        assert_eq!(info.percentage(), 99);
    }

    #[test]
    fn test_progress_bar_parser() {
        let parser = parsers::ProgressBarParser;

        assert_eq!(
            parser.parse("[=====>    ] 50%").map(|p| p.percentage()),
            Some(50)
        );
        assert_eq!(
            parser.parse("[##########] 100%").map(|p| p.percentage()),
            Some(100)
        );
    }

    #[test]
    fn test_progress_bar_parser_variants() {
        let parser = parsers::ProgressBarParser;

        // Different bar characters
        assert_eq!(
            parser.parse("[----------] 0%").map(|p| p.percentage()),
            Some(0)
        );
        assert_eq!(
            parser.parse("[###-------] 30%").map(|p| p.percentage()),
            Some(30)
        );
        assert_eq!(
            parser.parse("[>         ] 10%").map(|p| p.percentage()),
            Some(10)
        );
    }

    #[test]
    fn test_composite_parser() {
        let parser = parsers::CompositeParser::default_all();

        assert_eq!(parser.parse("50%").map(|p| p.percentage()), Some(50));
        assert_eq!(parser.parse("5/10").map(|p| p.percentage()), Some(50));
        assert_eq!(
            parser.parse("[=====>    ] 50%").map(|p| p.percentage()),
            Some(50)
        );
    }

    #[test]
    fn test_composite_parser_priority() {
        let parser = parsers::CompositeParser::default_all();

        // Percentage parser has priority
        let info = parser.parse("Step 3/5: 60% complete");
        assert_eq!(info.map(|p| p.percentage()), Some(60));

        // When no percentage, fraction is used
        let info = parser.parse("Processing file 3/5");
        assert_eq!(info.map(|p| p.percentage()), Some(60));
    }

    #[test]
    fn test_composite_parser_no_match() {
        let parser = parsers::CompositeParser::default_all();
        assert!(parser.parse("Just some text").is_none());
        assert!(parser.parse("").is_none());
    }

    #[test]
    fn test_custom_composite_parser() {
        let parser = parsers::CompositeParser::new()
            .add(parsers::FractionParser)
            .add(parsers::PercentageParser);

        // Fraction has priority now
        let info = parser.parse("Step 3/5: 60% complete");
        assert_eq!(info.map(|p| p.percentage()), Some(60)); // 3/5 = 60%
    }

    // ==================== ProgressInfo Tests ====================

    #[test]
    fn test_progress_info() {
        let info = ProgressInfo::new(50, 100);
        assert_eq!(info.percentage(), 50);

        let info = ProgressInfo::new(0, 0);
        assert_eq!(info.percentage(), 0);

        let info = ProgressInfo::with_message(75, 100, "Almost done");
        assert_eq!(info.percentage(), 75);
        assert_eq!(info.message, Some("Almost done".to_string()));
    }

    #[test]
    fn test_progress_info_edge_cases() {
        // Zero total
        let info = ProgressInfo::new(50, 0);
        assert_eq!(info.percentage(), 0);

        // Current > Total
        let info = ProgressInfo::new(150, 100);
        assert_eq!(info.percentage(), 100);

        // Large numbers
        let info = ProgressInfo::new(500000, 1000000);
        assert_eq!(info.percentage(), 50);
    }

    #[test]
    fn test_progress_info_serialization() {
        let info = ProgressInfo::with_message(50, 100, "Halfway");
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("50"));
        assert!(json.contains("100"));
        assert!(json.contains("Halfway"));

        let deserialized: ProgressInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.current, 50);
        assert_eq!(deserialized.total, 100);
        assert_eq!(deserialized.message, Some("Halfway".to_string()));
    }

    // ==================== CliBridgeConfig Tests ====================

    #[test]
    fn test_cli_bridge_config() {
        let config = CliBridgeConfig::default();
        assert!(config.auto_register);
        assert!(config.capture_stdout);
        assert!(config.capture_stderr);

        let config = CliBridgeConfig::with_server("/tmp/test.sock");
        assert_eq!(config.server_url, "/tmp/test.sock");

        let config = CliBridgeConfig::default().no_auto_register();
        assert!(!config.auto_register);
    }

    #[test]
    fn test_cli_bridge_config_builder() {
        let config = CliBridgeConfig::default()
            .no_auto_register()
            .progress_parser(parsers::PercentageParser);

        assert!(!config.auto_register);
        assert!(config.progress_parser.is_some());
    }

    #[test]
    fn test_cli_bridge_config_debug() {
        let config = CliBridgeConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("CliBridgeConfig"));
        assert!(debug_str.contains("auto_register"));
    }

    // ==================== CliBridge Tests ====================

    #[test]
    fn test_cli_bridge_creation() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        assert!(bridge.task_id().is_none());
        assert!(!bridge.is_cancelled());
    }

    #[test]
    fn test_cli_bridge_register_task() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        let task_id = bridge.register_task("Test Task", "test").unwrap();

        assert!(task_id.starts_with("cli-"));
        assert_eq!(bridge.task_id(), Some(task_id));
    }

    #[test]
    fn test_cli_bridge_progress() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        bridge.register_task("Test", "test").unwrap();

        bridge.set_progress(50, Some("Halfway"));
        // Progress is stored internally
        let state = bridge.state.read();
        assert_eq!(state.progress, 50);
        assert_eq!(state.progress_message, Some("Halfway".to_string()));
    }

    #[test]
    fn test_cli_bridge_progress_clamping() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        bridge.register_task("Test", "test").unwrap();

        // Progress should be clamped to 100
        bridge.set_progress(150, None);
        let state = bridge.state.read();
        assert_eq!(state.progress, 100);
    }

    #[test]
    fn test_cli_bridge_cancellation() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        assert!(!bridge.is_cancelled());

        // Get cancel token and cancel
        let token = bridge.cancel_token();
        token.cancel();
        assert!(bridge.is_cancelled());
    }

    #[test]
    fn test_cli_bridge_complete() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        bridge.register_task("Test", "test").unwrap();

        bridge.complete(serde_json::json!({"success": true}));

        let state = bridge.state.read();
        assert!(state.completed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_cli_bridge_fail() {
        let bridge = CliBridge::new(CliBridgeConfig::default()).unwrap();
        bridge.register_task("Test", "test").unwrap();

        bridge.fail("Something went wrong");

        let state = bridge.state.read();
        assert!(state.completed.load(std::sync::atomic::Ordering::SeqCst));
    }

    // ==================== WrappedCommand Tests ====================

    #[test]
    fn test_wrapped_command_creation() {
        let cmd = WrappedCommand::new("echo")
            .arg("hello")
            .task("Echo Test", "test");

        assert_eq!(cmd.task_name, "Echo Test");
        assert_eq!(cmd.task_type, "test");
    }

    #[test]
    fn test_wrapped_command_builder() {
        let cmd = WrappedCommand::new("cargo")
            .args(["build", "--release"])
            .task("Build", "build")
            .progress_parser(parsers::PercentageParser);

        assert_eq!(cmd.task_name, "Build");
        assert_eq!(cmd.task_type, "build");
        assert!(cmd.progress_parser.is_some());
    }

    #[test]
    fn test_wrapped_command_env() {
        let cmd = WrappedCommand::new("echo")
            .env("MY_VAR", "my_value")
            .env("ANOTHER_VAR", "another_value");

        // Just verify it builds without error
        assert_eq!(cmd.task_type, "command");
    }

    #[test]
    fn test_wrapped_command_current_dir() {
        let cmd = WrappedCommand::new("echo").current_dir(std::path::Path::new("/tmp"));

        assert_eq!(cmd.task_type, "command");
    }

    #[cfg(windows)]
    #[test]
    fn test_wrapped_command_run_echo() {
        let output = WrappedCommand::new("cmd")
            .args(["/C", "echo", "hello"])
            .task("Echo Test", "test")
            .run()
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_wrapped_command_run_echo() {
        let output = WrappedCommand::new("echo")
            .arg("hello")
            .task("Echo Test", "test")
            .run()
            .unwrap();

        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("hello"));
    }

    #[cfg(windows)]
    #[test]
    fn test_wrapped_command_run_failure() {
        let output = WrappedCommand::new("cmd")
            .args(["/C", "exit", "1"])
            .task("Fail Test", "test")
            .run()
            .unwrap();

        assert_eq!(output.exit_code, 1);
    }

    #[cfg(not(windows))]
    #[test]
    fn test_wrapped_command_run_failure() {
        let output = WrappedCommand::new("sh")
            .args(["-c", "exit 1"])
            .task("Fail Test", "test")
            .run()
            .unwrap();

        assert_eq!(output.exit_code, 1);
    }

    // ==================== CommandOutput Tests ====================

    #[test]
    fn test_command_output_debug() {
        let output = CommandOutput {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
        };

        let debug_str = format!("{:?}", output);
        assert!(debug_str.contains("exit_code"));
        assert!(debug_str.contains("0"));
    }

    // ==================== WrappedWriter Tests ====================

    #[test]
    fn test_wrapped_writer_stdout() {
        let state = Arc::new(RwLock::new(BridgeState::default()));
        let mut writer = WrappedWriter::new(
            "/tmp/test.sock".to_string(),
            Some("test-task".to_string()),
            OutputType::Stdout,
            Some(Arc::new(parsers::PercentageParser)),
            Arc::clone(&state),
        );

        // Write a line with progress
        let data = b"Progress: 50%\n";
        let written = writer.write(data).unwrap();
        assert_eq!(written, data.len());

        // Check progress was parsed
        let s = state.read();
        assert_eq!(s.progress, 50);
    }

    #[test]
    fn test_wrapped_writer_stderr() {
        let state = Arc::new(RwLock::new(BridgeState::default()));
        let mut writer = WrappedWriter::new(
            "/tmp/test.sock".to_string(),
            Some("test-task".to_string()),
            OutputType::Stderr,
            None,
            Arc::clone(&state),
        );

        let data = b"Error message\n";
        let written = writer.write(data).unwrap();
        assert_eq!(written, data.len());
    }

    #[test]
    fn test_wrapped_writer_buffering() {
        let state = Arc::new(RwLock::new(BridgeState::default()));
        let mut writer = WrappedWriter::new(
            "/tmp/test.sock".to_string(),
            Some("test-task".to_string()),
            OutputType::Stdout,
            Some(Arc::new(parsers::PercentageParser)),
            Arc::clone(&state),
        );

        // Write partial line
        writer.write_all(b"Progress: ").unwrap();
        assert_eq!(state.read().progress, 0);

        // Complete the line
        writer.write_all(b"75%\n").unwrap();
        assert_eq!(state.read().progress, 75);
    }

    #[test]
    fn test_wrapped_writer_flush() {
        let state = Arc::new(RwLock::new(BridgeState::default()));
        let mut writer = WrappedWriter::new(
            "/tmp/test.sock".to_string(),
            Some("test-task".to_string()),
            OutputType::Stdout,
            Some(Arc::new(parsers::PercentageParser)),
            Arc::clone(&state),
        );

        // Write without newline
        writer.write_all(b"Progress: 90%").unwrap();
        assert_eq!(state.read().progress, 0);

        // Flush should process remaining buffer
        writer.flush().unwrap();
        assert_eq!(state.read().progress, 90);
    }

    // ==================== OutputType Tests ====================

    #[test]
    fn test_output_type_equality() {
        assert_eq!(OutputType::Stdout, OutputType::Stdout);
        assert_eq!(OutputType::Stderr, OutputType::Stderr);
        assert_ne!(OutputType::Stdout, OutputType::Stderr);
    }

    #[test]
    fn test_output_type_debug() {
        assert_eq!(format!("{:?}", OutputType::Stdout), "Stdout");
        assert_eq!(format!("{:?}", OutputType::Stderr), "Stderr");
    }
}
