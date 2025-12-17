//! Python bindings for TaskManager

use crate::bindings::json_utils::{json_value_to_py, py_to_json_value};
use crate::task_manager::{
    CancellationToken, TaskBuilder, TaskFilter, TaskHandle, TaskInfo, TaskManager,
    TaskManagerConfig, TaskStatus,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

/// Python wrapper for TaskStatus.
#[pyclass(name = "TaskStatus", eq)]
#[derive(Clone, PartialEq)]
pub struct PyTaskStatus {
    inner: TaskStatus,
}

#[pymethods]
impl PyTaskStatus {
    /// Pending status.
    #[classattr]
    #[allow(non_snake_case)]
    fn PENDING() -> Self {
        Self {
            inner: TaskStatus::Pending,
        }
    }

    /// Running status.
    #[classattr]
    #[allow(non_snake_case)]
    fn RUNNING() -> Self {
        Self {
            inner: TaskStatus::Running,
        }
    }

    /// Paused status.
    #[classattr]
    #[allow(non_snake_case)]
    fn PAUSED() -> Self {
        Self {
            inner: TaskStatus::Paused,
        }
    }

    /// Completed status.
    #[classattr]
    #[allow(non_snake_case)]
    fn COMPLETED() -> Self {
        Self {
            inner: TaskStatus::Completed,
        }
    }

    /// Failed status.
    #[classattr]
    #[allow(non_snake_case)]
    fn FAILED() -> Self {
        Self {
            inner: TaskStatus::Failed,
        }
    }

    /// Cancelled status.
    #[classattr]
    #[allow(non_snake_case)]
    fn CANCELLED() -> Self {
        Self {
            inner: TaskStatus::Cancelled,
        }
    }

    /// Check if the task is in a terminal state.
    fn is_terminal(&self) -> bool {
        self.inner.is_terminal()
    }

    /// Check if the task is active.
    fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    fn __repr__(&self) -> String {
        format!("TaskStatus.{:?}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.inner).to_lowercase()
    }
}

impl From<TaskStatus> for PyTaskStatus {
    fn from(status: TaskStatus) -> Self {
        Self { inner: status }
    }
}

/// Python wrapper for TaskInfo.
#[pyclass(name = "TaskInfo")]
#[derive(Clone)]
pub struct PyTaskInfo {
    inner: TaskInfo,
}

#[pymethods]
impl PyTaskInfo {
    /// Get the task ID.
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Get the task name.
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the task type.
    #[getter]
    fn task_type(&self) -> &str {
        &self.inner.task_type
    }

    /// Get the task status.
    #[getter]
    fn status(&self) -> PyTaskStatus {
        self.inner.status.into()
    }

    /// Get the progress (0-100).
    #[getter]
    fn progress(&self) -> u8 {
        self.inner.progress
    }

    /// Get the progress message.
    #[getter]
    fn progress_message(&self) -> Option<&str> {
        self.inner.progress_message.as_deref()
    }

    /// Get the creation time as Unix timestamp.
    #[getter]
    fn created_at(&self) -> f64 {
        self.inner
            .created_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64()
    }

    /// Get the start time as Unix timestamp.
    #[getter]
    fn started_at(&self) -> Option<f64> {
        self.inner.started_at.map(|t| {
            t.duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs_f64()
        })
    }

    /// Get the finish time as Unix timestamp.
    #[getter]
    fn finished_at(&self) -> Option<f64> {
        self.inner.finished_at.map(|t| {
            t.duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_secs_f64()
        })
    }

    /// Get the error message if failed.
    #[getter]
    fn error(&self) -> Option<&str> {
        self.inner.error.as_deref()
    }

    /// Get the result data if completed.
    #[getter]
    fn result(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match &self.inner.result {
            Some(v) => Ok(Some(json_value_to_py(py, v)?)),
            None => Ok(None),
        }
    }

    /// Get metadata as a dict.
    fn get_metadata(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = pyo3::types::PyDict::new(py);
        for (k, v) in &self.inner.metadata {
            dict.set_item(k, json_value_to_py(py, v)?)?;
        }
        Ok(dict.into())
    }

    /// Get labels as a dict.
    fn get_labels(&self) -> std::collections::HashMap<String, String> {
        self.inner.labels.clone()
    }

    /// Convert to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskInfo(id='{}', name='{}', status={:?}, progress={})",
            self.inner.id, self.inner.name, self.inner.status, self.inner.progress
        )
    }
}

/// Python wrapper for CancellationToken.
#[pyclass(name = "CancellationToken")]
#[derive(Clone)]
pub struct PyCancellationToken {
    inner: CancellationToken,
}

#[pymethods]
impl PyCancellationToken {
    /// Create a new cancellation token.
    #[new]
    fn new() -> Self {
        Self {
            inner: CancellationToken::new(),
        }
    }

    /// Trigger cancellation.
    fn cancel(&self) {
        self.inner.cancel();
    }

    /// Check if cancellation has been requested.
    #[getter]
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Create a child token.
    fn child(&self) -> Self {
        Self {
            inner: self.inner.child(),
        }
    }

    fn __repr__(&self) -> String {
        format!("CancellationToken(cancelled={})", self.inner.is_cancelled())
    }
}

/// Python wrapper for TaskBuilder.
#[pyclass(name = "TaskBuilder")]
#[derive(Clone)]
pub struct PyTaskBuilder {
    inner: TaskBuilder,
}

#[pymethods]
impl PyTaskBuilder {
    /// Create a new task builder.
    #[new]
    fn new(name: &str, task_type: &str) -> Self {
        Self {
            inner: TaskBuilder::new(name, task_type),
        }
    }

    /// Add metadata to the task.
    fn metadata(&self, py: Python<'_>, key: &str, value: Py<PyAny>) -> PyResult<Self> {
        let json_value = py_to_json_value(&value.bind(py).clone())?;
        Ok(Self {
            inner: self.inner.clone().metadata(key, json_value),
        })
    }

    /// Add a label to the task.
    fn label(&self, key: &str, value: &str) -> Self {
        Self {
            inner: self.inner.clone().label(key, value),
        }
    }

    fn __repr__(&self) -> String {
        "TaskBuilder(...)".to_string()
    }
}

/// Python wrapper for TaskFilter.
#[pyclass(name = "TaskFilter")]
#[derive(Clone)]
pub struct PyTaskFilter {
    inner: TaskFilter,
}

#[pymethods]
impl PyTaskFilter {
    /// Create a new empty filter.
    #[new]
    fn new() -> Self {
        Self {
            inner: TaskFilter::new(),
        }
    }

    /// Filter by status.
    fn status(&self, status: &PyTaskStatus) -> Self {
        Self {
            inner: self.inner.clone().status(status.inner),
        }
    }

    /// Filter by task type.
    fn task_type(&self, t: &str) -> Self {
        Self {
            inner: self.inner.clone().task_type(t),
        }
    }

    /// Filter by label.
    fn label(&self, key: &str, value: &str) -> Self {
        Self {
            inner: self.inner.clone().label(key, value),
        }
    }

    /// Show only active tasks.
    fn active(&self) -> Self {
        Self {
            inner: self.inner.clone().active(),
        }
    }

    /// Check if a task matches this filter.
    fn matches(&self, info: &PyTaskInfo) -> bool {
        self.inner.matches(&info.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskFilter(active_only={}, task_type={:?})",
            self.inner.active_only, self.inner.task_type
        )
    }
}

/// Python wrapper for TaskHandle.
#[pyclass(name = "TaskHandle")]
pub struct PyTaskHandle {
    inner: TaskHandle,
}

#[pymethods]
impl PyTaskHandle {
    /// Get the task ID.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id()
    }

    /// Get current task information.
    fn info(&self) -> PyTaskInfo {
        PyTaskInfo {
            inner: self.inner.info(),
        }
    }

    /// Get the current status.
    #[getter]
    fn status(&self) -> PyTaskStatus {
        self.inner.status().into()
    }

    /// Get the current progress.
    #[getter]
    fn progress(&self) -> u8 {
        self.inner.progress()
    }

    /// Update the task progress.
    #[pyo3(signature = (progress, message=None))]
    fn set_progress(&self, progress: u8, message: Option<&str>) {
        self.inner.set_progress(progress, message);
    }

    /// Publish a log message.
    fn log(&self, level: &str, message: &str) {
        self.inner.log(level, message);
    }

    /// Publish stdout output.
    fn stdout(&self, line: &str) {
        self.inner.stdout(line);
    }

    /// Publish stderr output.
    fn stderr(&self, line: &str) {
        self.inner.stderr(line);
    }

    /// Check if cancellation has been requested.
    #[getter]
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    /// Get the cancellation token.
    fn cancel_token(&self) -> PyCancellationToken {
        PyCancellationToken {
            inner: self.inner.cancel_token(),
        }
    }

    /// Mark the task as started.
    fn start(&self) {
        self.inner.start();
    }

    /// Mark the task as completed with a result.
    fn complete(&self, py: Python<'_>, result: Option<Py<PyAny>>) -> PyResult<()> {
        let json_result = match result {
            Some(obj) => py_to_json_value(&obj.bind(py).clone())?,
            None => serde_json::json!({}),
        };
        self.inner.complete(json_result);
        Ok(())
    }

    /// Mark the task as failed with an error.
    fn fail(&self, error: &str) {
        self.inner.fail(error);
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskHandle(id='{}', status={:?}, progress={})",
            self.inner.id(),
            self.inner.status(),
            self.inner.progress()
        )
    }
}

/// Python wrapper for TaskManagerConfig.
#[pyclass(name = "TaskManagerConfig")]
#[derive(Clone)]
pub struct PyTaskManagerConfig {
    inner: TaskManagerConfig,
}

#[pymethods]
impl PyTaskManagerConfig {
    /// Create a new task manager configuration.
    #[new]
    #[pyo3(signature = (retention_seconds=3600, max_concurrent=100))]
    fn new(retention_seconds: u64, max_concurrent: usize) -> Self {
        Self {
            inner: TaskManagerConfig {
                retention_period: Duration::from_secs(retention_seconds),
                max_concurrent,
                ..Default::default()
            },
        }
    }

    /// Get the retention period in seconds.
    #[getter]
    fn retention_seconds(&self) -> u64 {
        self.inner.retention_period.as_secs()
    }

    /// Get the maximum concurrent tasks.
    #[getter]
    fn max_concurrent(&self) -> usize {
        self.inner.max_concurrent
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskManagerConfig(retention_seconds={}, max_concurrent={})",
            self.inner.retention_period.as_secs(),
            self.inner.max_concurrent
        )
    }
}

/// Python wrapper for TaskManager.
#[pyclass(name = "TaskManager")]
pub struct PyTaskManager {
    inner: Arc<TaskManager>,
}

#[pymethods]
impl PyTaskManager {
    /// Create a new task manager.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<PyTaskManagerConfig>) -> Self {
        let cfg = config.map(|c| c.inner).unwrap_or_default();
        Self {
            inner: Arc::new(TaskManager::new(cfg)),
        }
    }

    /// Create a new task.
    fn create(&self, builder: &PyTaskBuilder) -> PyTaskHandle {
        PyTaskHandle {
            inner: self.inner.create(builder.inner.clone()),
        }
    }

    /// Create a task with name and type directly.
    fn create_task(&self, name: &str, task_type: &str) -> PyTaskHandle {
        PyTaskHandle {
            inner: self.inner.create(TaskBuilder::new(name, task_type)),
        }
    }

    /// Get task information by ID.
    fn get(&self, id: &str) -> Option<PyTaskInfo> {
        self.inner.get(id).map(|info| PyTaskInfo { inner: info })
    }

    /// Get a task handle by ID.
    fn get_handle(&self, id: &str) -> Option<PyTaskHandle> {
        self.inner
            .get_handle(id)
            .map(|handle| PyTaskHandle { inner: handle })
    }

    /// List tasks matching the filter.
    #[pyo3(signature = (filter=None))]
    fn list(&self, filter: Option<&PyTaskFilter>) -> Vec<PyTaskInfo> {
        let f = filter.map(|f| f.inner.clone()).unwrap_or_default();
        self.inner
            .list(&f)
            .into_iter()
            .map(|info| PyTaskInfo { inner: info })
            .collect()
    }

    /// List all active tasks.
    fn list_active(&self) -> Vec<PyTaskInfo> {
        self.inner
            .list(&TaskFilter::new().active())
            .into_iter()
            .map(|info| PyTaskInfo { inner: info })
            .collect()
    }

    /// Cancel a task.
    fn cancel(&self, id: &str) -> PyResult<()> {
        self.inner
            .cancel(id)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Pause a task.
    fn pause(&self, id: &str) -> PyResult<()> {
        self.inner
            .pause(id)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Resume a paused task.
    fn resume(&self, id: &str) -> PyResult<()> {
        self.inner
            .resume(id)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Remove a completed task.
    fn remove(&self, id: &str) -> PyResult<()> {
        self.inner
            .remove(id)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Cleanup expired tasks.
    fn cleanup(&self) {
        self.inner.cleanup();
    }

    fn __repr__(&self) -> String {
        "TaskManager()".to_string()
    }
}
