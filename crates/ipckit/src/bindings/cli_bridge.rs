//! Python bindings for CLI Bridge
//!
//! This module provides Python bindings for the CLI Bridge functionality.

use crate::cli_bridge::{
    parsers, CliBridge, CliBridgeConfig, ProgressInfo, ProgressParser, WrappedCommand,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::json_utils::py_to_json_value;

/// Python wrapper for CliBridgeConfig
#[pyclass(name = "CliBridgeConfig")]
#[derive(Clone)]
pub struct PyCliBridgeConfig {
    inner: CliBridgeConfig,
}

#[pymethods]
impl PyCliBridgeConfig {
    /// Create a new configuration with default values.
    #[new]
    #[pyo3(signature = (server_url=None, auto_register=true, capture_stdout=true, capture_stderr=true))]
    fn new(
        server_url: Option<String>,
        auto_register: bool,
        capture_stdout: bool,
        capture_stderr: bool,
    ) -> Self {
        let mut config = CliBridgeConfig::default();
        if let Some(url) = server_url {
            config.server_url = url;
        }
        config.auto_register = auto_register;
        config.capture_stdout = capture_stdout;
        config.capture_stderr = capture_stderr;

        Self { inner: config }
    }

    /// Create configuration from environment variables.
    #[staticmethod]
    fn from_env() -> Self {
        Self {
            inner: CliBridgeConfig::from_env(),
        }
    }

    /// Get the server URL.
    #[getter]
    fn server_url(&self) -> &str {
        &self.inner.server_url
    }

    /// Set the server URL.
    #[setter]
    fn set_server_url(&mut self, url: String) {
        self.inner.server_url = url;
    }

    /// Get auto_register setting.
    #[getter]
    fn auto_register(&self) -> bool {
        self.inner.auto_register
    }

    /// Set auto_register.
    #[setter]
    fn set_auto_register(&mut self, value: bool) {
        self.inner.auto_register = value;
    }
}

/// Python wrapper for ProgressInfo
#[pyclass(name = "ProgressInfo")]
#[derive(Clone)]
pub struct PyProgressInfo {
    inner: ProgressInfo,
}

#[pymethods]
impl PyProgressInfo {
    /// Create a new progress info.
    #[new]
    #[pyo3(signature = (current, total, message=None))]
    fn new(current: u64, total: u64, message: Option<String>) -> Self {
        let inner = if let Some(msg) = message {
            ProgressInfo::with_message(current, total, &msg)
        } else {
            ProgressInfo::new(current, total)
        };
        Self { inner }
    }

    /// Get current value.
    #[getter]
    fn current(&self) -> u64 {
        self.inner.current
    }

    /// Get total value.
    #[getter]
    fn total(&self) -> u64 {
        self.inner.total
    }

    /// Get message.
    #[getter]
    fn message(&self) -> Option<String> {
        self.inner.message.clone()
    }

    /// Get percentage (0-100).
    #[getter]
    fn percentage(&self) -> u8 {
        self.inner.percentage()
    }

    fn __repr__(&self) -> String {
        format!(
            "ProgressInfo(current={}, total={}, percentage={}%)",
            self.inner.current,
            self.inner.total,
            self.inner.percentage()
        )
    }
}

/// Python wrapper for CliBridge
#[pyclass(name = "CliBridge")]
pub struct PyCliBridge {
    inner: CliBridge,
}

#[pymethods]
impl PyCliBridge {
    /// Create a new CLI bridge with default configuration.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<PyCliBridgeConfig>) -> PyResult<Self> {
        let config = config.map(|c| c.inner).unwrap_or_default();
        let inner = CliBridge::new(config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Connect with default configuration (from environment).
    #[staticmethod]
    fn connect() -> PyResult<Self> {
        let inner = CliBridge::connect()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Connect with the given configuration.
    #[staticmethod]
    fn connect_with_config(config: PyCliBridgeConfig) -> PyResult<Self> {
        let inner = CliBridge::connect_with_config(config.inner)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Register the current process as a task.
    fn register_task(&self, name: &str, task_type: &str) -> PyResult<String> {
        self.inner
            .register_task(name, task_type)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Get the current task ID.
    #[getter]
    fn task_id(&self) -> Option<String> {
        self.inner.task_id()
    }

    /// Set progress (0-100).
    #[pyo3(signature = (progress, message=None))]
    fn set_progress(&self, progress: u8, message: Option<&str>) {
        self.inner.set_progress(progress, message);
    }

    /// Log a message.
    fn log(&self, level: &str, message: &str) {
        self.inner.log(level, message);
    }

    /// Send stdout line.
    fn stdout(&self, line: &str) {
        self.inner.stdout(line);
    }

    /// Send stderr line.
    fn stderr(&self, line: &str) {
        self.inner.stderr(line);
    }

    /// Check if cancellation has been requested.
    #[getter]
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Mark the task as complete.
    fn complete(&self, py: Python<'_>, result: Py<PyAny>) -> PyResult<()> {
        let value = py_to_json_value(result.bind(py))?;
        self.inner.complete(value);
        Ok(())
    }

    /// Mark the task as failed.
    fn fail(&self, error: &str) {
        self.inner.fail(error);
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<Py<PyAny>>,
        exc_value: Option<Py<PyAny>>,
        _traceback: Option<Py<PyAny>>,
    ) {
        if exc_value.is_some() {
            self.inner.fail("Exception occurred");
        }
    }
}

/// Python wrapper for CommandOutput
#[pyclass(name = "CommandOutput")]
pub struct PyCommandOutput {
    /// Exit code
    #[pyo3(get)]
    pub exit_code: i32,
    /// Captured stdout
    #[pyo3(get)]
    pub stdout: String,
    /// Captured stderr
    #[pyo3(get)]
    pub stderr: String,
    /// Duration in milliseconds
    #[pyo3(get)]
    pub duration_ms: u64,
}

#[pymethods]
impl PyCommandOutput {
    fn __repr__(&self) -> String {
        format!(
            "CommandOutput(exit_code={}, duration_ms={})",
            self.exit_code, self.duration_ms
        )
    }

    /// Check if the command succeeded.
    #[getter]
    fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Wrap a command for execution with CLI bridge integration.
///
/// Args:
///     args: Command and arguments as a list
///     task_name: Name of the task
///     task_type: Type of the task
///     cwd: Working directory (optional)
///     env: Environment variables (optional)
///
/// Returns:
///     CommandOutput with exit code, stdout, stderr, and duration
#[pyfunction]
#[pyo3(signature = (args, task_name=None, task_type=None, cwd=None, env=None))]
pub fn wrap_command(
    py: Python<'_>,
    args: Vec<String>,
    task_name: Option<String>,
    task_type: Option<String>,
    cwd: Option<String>,
    env: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyCommandOutput> {
    if args.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "args must not be empty",
        ));
    }

    let program = &args[0];
    let mut cmd = WrappedCommand::new(program);

    // Add arguments
    if args.len() > 1 {
        cmd = cmd.args(&args[1..]);
    }

    // Set task info
    if let Some(name) = task_name {
        let t = task_type.unwrap_or_else(|| "command".to_string());
        cmd = cmd.task(&name, &t);
    }

    // Set working directory
    if let Some(dir) = cwd {
        cmd = cmd.current_dir(std::path::Path::new(&dir));
    }

    // Set environment variables
    if let Some(env_dict) = env {
        for (key, value) in env_dict.iter() {
            let k: String = key.extract()?;
            let v: String = value.extract()?;
            cmd = cmd.env(&k, &v);
        }
    }

    // Add default progress parser
    cmd = cmd.progress_parser(parsers::CompositeParser::default_all());

    // Run the command (release GIL during execution)
    let output = py
        .detach(|| cmd.run())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyCommandOutput {
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        duration_ms: output.duration.as_millis() as u64,
    })
}

/// Parse progress from a line using built-in parsers.
///
/// Args:
///     line: The line to parse
///     parser_type: Parser type ("percentage", "fraction", "progress_bar", or "all")
///
/// Returns:
///     ProgressInfo if progress was found, None otherwise
#[pyfunction]
#[pyo3(signature = (line, parser_type="all"))]
pub fn parse_progress(line: &str, parser_type: &str) -> Option<PyProgressInfo> {
    let parser: Box<dyn ProgressParser> = match parser_type {
        "percentage" => Box::new(parsers::PercentageParser),
        "fraction" => Box::new(parsers::FractionParser),
        "progress_bar" => Box::new(parsers::ProgressBarParser),
        _ => Box::new(parsers::CompositeParser::default_all()),
    };

    parser
        .parse(line)
        .map(|info| PyProgressInfo { inner: info })
}
