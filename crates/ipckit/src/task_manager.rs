//! Task Manager - Task Registration and Lifecycle Management
//!
//! This module provides a task manager for registering, tracking, and managing
//! long-running tasks, similar to Docker container management and kubectl Job management.
//!
//! # Features
//!
//! - Task lifecycle management (create, start, pause, resume, cancel, complete)
//! - Task discovery and filtering
//! - Real-time progress and log monitoring
//! - Cooperative cancellation with cancellation tokens
//!
//! # Example
//!
//! ```rust,no_run
//! use ipckit::{TaskManager, TaskBuilder, TaskFilter, TaskStatus};
//! use std::time::Duration;
//!
//! let manager = TaskManager::new(Default::default());
//!
//! // Create and run a task
//! let handle = manager.create(TaskBuilder::new("Upload files", "upload"));
//!
//! // Update progress
//! handle.set_progress(50, Some("Half done"));
//!
//! // Check cancellation
//! if !handle.is_cancelled() {
//!     // Do work...
//!     handle.complete(serde_json::json!({"uploaded": 10}));
//! }
//!
//! // List active tasks
//! let active = manager.list(&TaskFilter::new().active());
//! ```

use crate::error::{IpcError, Result};
use crate::event_stream::{event_types, Event, EventBus, EventBusConfig, EventPublisher};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Task status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Waiting to execute
    Pending,
    /// Currently running
    Running,
    /// Paused
    Paused,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

impl TaskStatus {
    /// Check if the task is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the task is active (pending or running).
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::Running | Self::Paused)
    }
}

impl From<u8> for TaskStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Pending,
            1 => Self::Running,
            2 => Self::Paused,
            3 => Self::Completed,
            4 => Self::Failed,
            5 => Self::Cancelled,
            _ => Self::Pending,
        }
    }
}

impl From<TaskStatus> for u8 {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Pending => 0,
            TaskStatus::Running => 1,
            TaskStatus::Paused => 2,
            TaskStatus::Completed => 3,
            TaskStatus::Failed => 4,
            TaskStatus::Cancelled => 5,
        }
    }
}

/// Task information structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Task ID
    pub id: String,
    /// Task name
    pub name: String,
    /// Task type (e.g., "upload", "download", "build")
    pub task_type: String,
    /// Task status
    pub status: TaskStatus,
    /// Progress (0-100)
    pub progress: u8,
    /// Progress message
    pub progress_message: Option<String>,
    /// Creation time
    #[serde(with = "system_time_serde")]
    pub created_at: SystemTime,
    /// Start time
    #[serde(with = "option_system_time_serde")]
    pub started_at: Option<SystemTime>,
    /// Completion time
    #[serde(with = "option_system_time_serde")]
    pub finished_at: Option<SystemTime>,
    /// Task metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Task labels
    pub labels: HashMap<String, String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Result data (if completed)
    pub result: Option<serde_json::Value>,
}

mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        duration.as_secs_f64().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs_f64(secs))
    }
}

mod option_system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &Option<SystemTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time {
            Some(t) => {
                let duration = t.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
                Some(duration.as_secs_f64()).serialize(serializer)
            }
            None => None::<f64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SystemTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<f64>::deserialize(deserializer)?;
        Ok(opt.map(|secs| UNIX_EPOCH + Duration::from_secs_f64(secs)))
    }
}

/// Cancellation token for cooperative task cancellation.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Trigger cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Create a child token that is cancelled when the parent is cancelled.
    pub fn child(&self) -> Self {
        // For simplicity, we share the same atomic
        // In a more sophisticated implementation, we'd have a hierarchy
        Self {
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}

/// Internal task state.
struct TaskState {
    info: RwLock<TaskInfo>,
    status: AtomicU8,
    progress: AtomicU8,
    cancel_token: CancellationToken,
}

impl TaskState {
    fn new(info: TaskInfo) -> Self {
        Self {
            status: AtomicU8::new(info.status.into()),
            progress: AtomicU8::new(info.progress),
            info: RwLock::new(info),
            cancel_token: CancellationToken::new(),
        }
    }

    fn get_info(&self) -> TaskInfo {
        let mut info = self.info.read().clone();
        info.status = TaskStatus::from(self.status.load(Ordering::SeqCst));
        info.progress = self.progress.load(Ordering::SeqCst);
        info
    }

    fn set_status(&self, status: TaskStatus) {
        self.status.store(status.into(), Ordering::SeqCst);
        self.info.write().status = status;
    }

    fn set_progress(&self, progress: u8, message: Option<&str>) {
        let progress = progress.min(100);
        self.progress.store(progress, Ordering::SeqCst);

        let mut info = self.info.write();
        info.progress = progress;
        if let Some(msg) = message {
            info.progress_message = Some(msg.to_string());
        }
    }
}

/// Task handle for controlling and monitoring a task.
#[derive(Clone)]
pub struct TaskHandle {
    id: String,
    state: Arc<TaskState>,
    publisher: EventPublisher,
}

impl TaskHandle {
    /// Get the task ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get current task information.
    pub fn info(&self) -> TaskInfo {
        self.state.get_info()
    }

    /// Get the current status.
    pub fn status(&self) -> TaskStatus {
        TaskStatus::from(self.state.status.load(Ordering::SeqCst))
    }

    /// Get the current progress.
    pub fn progress(&self) -> u8 {
        self.state.progress.load(Ordering::SeqCst)
    }

    /// Update the task progress.
    pub fn set_progress(&self, progress: u8, message: Option<&str>) {
        self.state.set_progress(progress, message);
        self.publisher
            .progress(&self.id, progress as u64, 100, message.unwrap_or(""));
    }

    /// Publish a log message.
    pub fn log(&self, level: &str, message: &str) {
        self.publisher.log(&self.id, level, message);
    }

    /// Publish stdout output.
    pub fn stdout(&self, line: &str) {
        self.publisher.stdout(&self.id, line);
    }

    /// Publish stderr output.
    pub fn stderr(&self, line: &str) {
        self.publisher.stderr(&self.id, line);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.state.cancel_token.is_cancelled()
    }

    /// Get the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.state.cancel_token.clone()
    }

    /// Mark the task as started.
    pub fn start(&self) {
        self.state.set_status(TaskStatus::Running);
        self.state.info.write().started_at = Some(SystemTime::now());
        self.publisher.task_started(&self.id, serde_json::json!({}));
    }

    /// Mark the task as completed with a result.
    pub fn complete(&self, result: serde_json::Value) {
        self.state.set_status(TaskStatus::Completed);
        self.state.set_progress(100, Some("Completed"));

        {
            let mut info = self.state.info.write();
            info.finished_at = Some(SystemTime::now());
            info.result = Some(result.clone());
        }

        self.publisher.task_completed(&self.id, result);
    }

    /// Mark the task as failed with an error.
    pub fn fail(&self, error: &str) {
        self.state.set_status(TaskStatus::Failed);

        {
            let mut info = self.state.info.write();
            info.finished_at = Some(SystemTime::now());
            info.error = Some(error.to_string());
        }

        self.publisher.task_failed(&self.id, error);
    }

    /// Get the event publisher for this task.
    pub fn publisher(&self) -> &EventPublisher {
        &self.publisher
    }
}

/// Builder for creating tasks.
#[derive(Debug, Clone)]
pub struct TaskBuilder {
    name: String,
    task_type: String,
    metadata: HashMap<String, serde_json::Value>,
    labels: HashMap<String, String>,
}

impl TaskBuilder {
    /// Create a new task builder.
    pub fn new(name: &str, task_type: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: task_type.to_string(),
            metadata: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    /// Add metadata to the task.
    pub fn metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    /// Add a label to the task.
    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
}

/// Task filter for querying tasks.
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    /// Filter by status
    pub status: Option<Vec<TaskStatus>>,
    /// Filter by task type
    pub task_type: Option<String>,
    /// Filter by labels
    pub labels: HashMap<String, String>,
    /// Show only active tasks
    pub active_only: bool,
}

impl TaskFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by status.
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status.get_or_insert_with(Vec::new).push(status);
        self
    }

    /// Filter by task type.
    pub fn task_type(mut self, t: &str) -> Self {
        self.task_type = Some(t.to_string());
        self
    }

    /// Filter by label.
    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }

    /// Show only active tasks.
    pub fn active(mut self) -> Self {
        self.active_only = true;
        self
    }

    /// Check if a task matches this filter.
    pub fn matches(&self, info: &TaskInfo) -> bool {
        // Check active only
        if self.active_only && !info.status.is_active() {
            return false;
        }

        // Check status
        if let Some(ref statuses) = self.status {
            if !statuses.contains(&info.status) {
                return false;
            }
        }

        // Check task type
        if let Some(ref t) = self.task_type {
            if info.task_type != *t {
                return false;
            }
        }

        // Check labels
        for (key, value) in &self.labels {
            match info.labels.get(key) {
                Some(v) if v == value => {}
                _ => return false,
            }
        }

        true
    }
}

/// Task manager configuration.
#[derive(Debug, Clone)]
pub struct TaskManagerConfig {
    /// Completed task retention period
    pub retention_period: Duration,
    /// Maximum concurrent tasks
    pub max_concurrent: usize,
    /// Event bus configuration
    pub event_bus_config: EventBusConfig,
}

impl Default for TaskManagerConfig {
    fn default() -> Self {
        Self {
            retention_period: Duration::from_secs(3600), // 1 hour
            max_concurrent: 100,
            event_bus_config: EventBusConfig::default(),
        }
    }
}

/// Task manager for creating and managing tasks.
pub struct TaskManager {
    tasks: RwLock<HashMap<String, Arc<TaskState>>>,
    event_bus: EventBus,
    config: TaskManagerConfig,
    next_id: AtomicU64,
}

impl TaskManager {
    /// Create a new task manager.
    pub fn new(config: TaskManagerConfig) -> Self {
        let event_bus = EventBus::new(config.event_bus_config.clone());

        Self {
            tasks: RwLock::new(HashMap::new()),
            event_bus,
            config,
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new task.
    pub fn create(&self, builder: TaskBuilder) -> TaskHandle {
        let id = format!("task-{}", self.next_id.fetch_add(1, Ordering::SeqCst));

        let info = TaskInfo {
            id: id.clone(),
            name: builder.name,
            task_type: builder.task_type,
            status: TaskStatus::Pending,
            progress: 0,
            progress_message: None,
            created_at: SystemTime::now(),
            started_at: None,
            finished_at: None,
            metadata: builder.metadata,
            labels: builder.labels,
            error: None,
            result: None,
        };

        let state = Arc::new(TaskState::new(info));
        self.tasks.write().insert(id.clone(), Arc::clone(&state));

        let publisher = self.event_bus.publisher();
        publisher.publish(Event::with_resource(
            event_types::TASK_CREATED,
            &id,
            serde_json::json!({}),
        ));

        TaskHandle {
            id,
            state,
            publisher,
        }
    }

    /// Spawn a task with a closure.
    pub fn spawn<F>(&self, name: &str, task_type: &str, f: F) -> TaskHandle
    where
        F: FnOnce(TaskHandle) + Send + 'static,
    {
        let handle = self.create(TaskBuilder::new(name, task_type));
        let handle_clone = handle.clone();

        std::thread::spawn(move || {
            handle_clone.start();
            f(handle_clone);
        });

        handle
    }

    /// Get task information by ID.
    pub fn get(&self, id: &str) -> Option<TaskInfo> {
        self.tasks.read().get(id).map(|s| s.get_info())
    }

    /// Get a task handle by ID.
    pub fn get_handle(&self, id: &str) -> Option<TaskHandle> {
        self.tasks.read().get(id).map(|state| TaskHandle {
            id: id.to_string(),
            state: Arc::clone(state),
            publisher: self.event_bus.publisher(),
        })
    }

    /// List tasks matching the filter.
    pub fn list(&self, filter: &TaskFilter) -> Vec<TaskInfo> {
        self.tasks
            .read()
            .values()
            .map(|s| s.get_info())
            .filter(|info| filter.matches(info))
            .collect()
    }

    /// Cancel a task.
    pub fn cancel(&self, id: &str) -> Result<()> {
        let tasks = self.tasks.read();
        let state = tasks
            .get(id)
            .ok_or_else(|| IpcError::NotFound(id.to_string()))?;

        state.cancel_token.cancel();
        state.set_status(TaskStatus::Cancelled);
        state.info.write().finished_at = Some(SystemTime::now());

        self.event_bus.publisher().task_cancelled(id);

        Ok(())
    }

    /// Pause a task.
    pub fn pause(&self, id: &str) -> Result<()> {
        let tasks = self.tasks.read();
        let state = tasks
            .get(id)
            .ok_or_else(|| IpcError::NotFound(id.to_string()))?;

        let current = TaskStatus::from(state.status.load(Ordering::SeqCst));
        if current != TaskStatus::Running {
            return Err(IpcError::InvalidState(format!(
                "Cannot pause task in {:?} state",
                current
            )));
        }

        state.set_status(TaskStatus::Paused);
        self.event_bus.publisher().publish(Event::with_resource(
            event_types::TASK_PAUSED,
            id,
            serde_json::json!({}),
        ));

        Ok(())
    }

    /// Resume a paused task.
    pub fn resume(&self, id: &str) -> Result<()> {
        let tasks = self.tasks.read();
        let state = tasks
            .get(id)
            .ok_or_else(|| IpcError::NotFound(id.to_string()))?;

        let current = TaskStatus::from(state.status.load(Ordering::SeqCst));
        if current != TaskStatus::Paused {
            return Err(IpcError::InvalidState(format!(
                "Cannot resume task in {:?} state",
                current
            )));
        }

        state.set_status(TaskStatus::Running);
        self.event_bus.publisher().publish(Event::with_resource(
            event_types::TASK_RESUMED,
            id,
            serde_json::json!({}),
        ));

        Ok(())
    }

    /// Remove a completed task from the manager.
    pub fn remove(&self, id: &str) -> Result<()> {
        let mut tasks = self.tasks.write();
        let state = tasks
            .get(id)
            .ok_or_else(|| IpcError::NotFound(id.to_string()))?;

        let status = TaskStatus::from(state.status.load(Ordering::SeqCst));
        if !status.is_terminal() {
            return Err(IpcError::InvalidState(format!(
                "Cannot remove task in {:?} state",
                status
            )));
        }

        tasks.remove(id);
        Ok(())
    }

    /// Cleanup expired tasks.
    pub fn cleanup(&self) {
        let now = SystemTime::now();
        let mut tasks = self.tasks.write();

        tasks.retain(|_, state| {
            let info = state.get_info();
            if !info.status.is_terminal() {
                return true;
            }

            if let Some(finished_at) = info.finished_at {
                if let Ok(elapsed) = now.duration_since(finished_at) {
                    return elapsed < self.config.retention_period;
                }
            }

            true
        });
    }

    /// Get the event bus for this manager.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get a publisher for the event bus.
    pub fn publisher(&self) -> EventPublisher {
        self.event_bus.publisher()
    }

    /// Get the number of tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.read().len()
    }

    /// Get the number of active tasks.
    pub fn active_task_count(&self) -> usize {
        self.tasks
            .read()
            .values()
            .filter(|s| TaskStatus::from(s.status.load(Ordering::SeqCst)).is_active())
            .count()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new(TaskManagerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_task_creation() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(TaskBuilder::new("Test Task", "test"));

        assert!(handle.id().starts_with("task-"));
        assert_eq!(handle.status(), TaskStatus::Pending);
        assert_eq!(handle.progress(), 0);
    }

    #[test]
    fn test_task_lifecycle() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(TaskBuilder::new("Test Task", "test"));

        // Start
        handle.start();
        assert_eq!(handle.status(), TaskStatus::Running);

        // Progress
        handle.set_progress(50, Some("Half done"));
        assert_eq!(handle.progress(), 50);

        // Complete
        handle.complete(serde_json::json!({"result": "success"}));
        assert_eq!(handle.status(), TaskStatus::Completed);
        assert_eq!(handle.progress(), 100);
    }

    #[test]
    fn test_task_cancellation() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(TaskBuilder::new("Test Task", "test"));

        handle.start();
        assert!(!handle.is_cancelled());

        manager.cancel(handle.id()).unwrap();
        assert!(handle.is_cancelled());
        assert_eq!(handle.status(), TaskStatus::Cancelled);
    }

    #[test]
    fn test_task_failure() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(TaskBuilder::new("Test Task", "test"));

        handle.start();
        handle.fail("Something went wrong");

        assert_eq!(handle.status(), TaskStatus::Failed);
        let info = handle.info();
        assert_eq!(info.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_task_filter() {
        let manager = TaskManager::new(Default::default());

        let h1 = manager.create(TaskBuilder::new("Task 1", "upload"));
        let h2 = manager.create(TaskBuilder::new("Task 2", "download"));
        let _h3 = manager.create(TaskBuilder::new("Task 3", "upload"));

        h1.start();
        h2.start();
        h2.complete(serde_json::json!({}));

        // Filter by type
        let uploads = manager.list(&TaskFilter::new().task_type("upload"));
        assert_eq!(uploads.len(), 2);

        // Filter active only
        let active = manager.list(&TaskFilter::new().active());
        assert_eq!(active.len(), 2); // h1 and h3

        // Filter by status
        let completed = manager.list(&TaskFilter::new().status(TaskStatus::Completed));
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_task_labels() {
        let manager = TaskManager::new(Default::default());

        let h1 = manager.create(
            TaskBuilder::new("Task 1", "test")
                .label("env", "prod")
                .label("priority", "high"),
        );

        let h2 = manager.create(TaskBuilder::new("Task 2", "test").label("env", "dev"));

        let prod_tasks = manager.list(&TaskFilter::new().label("env", "prod"));
        assert_eq!(prod_tasks.len(), 1);
        assert_eq!(prod_tasks[0].id, h1.id());

        let _ = h2; // Silence unused warning
    }

    #[test]
    fn test_task_metadata() {
        let manager = TaskManager::new(Default::default());

        let handle = manager.create(
            TaskBuilder::new("Task", "test")
                .metadata("file_count", serde_json::json!(10))
                .metadata("total_size", serde_json::json!(1024)),
        );

        let info = handle.info();
        assert_eq!(
            info.metadata.get("file_count"),
            Some(&serde_json::json!(10))
        );
        assert_eq!(
            info.metadata.get("total_size"),
            Some(&serde_json::json!(1024))
        );
    }

    #[test]
    fn test_task_spawn() {
        let manager = TaskManager::new(Default::default());

        let handle = manager.spawn("Spawned Task", "test", |h| {
            for i in 0..5 {
                if h.is_cancelled() {
                    return;
                }
                h.set_progress((i + 1) * 20, Some(&format!("Step {}", i + 1)));
                thread::sleep(Duration::from_millis(10));
            }
            h.complete(serde_json::json!({"done": true}));
        });

        // Wait for task to complete
        thread::sleep(Duration::from_millis(200));

        let info = handle.info();
        assert_eq!(info.status, TaskStatus::Completed);
        assert_eq!(info.progress, 100);
    }

    #[test]
    fn test_pause_resume() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(TaskBuilder::new("Task", "test"));

        handle.start();
        assert_eq!(handle.status(), TaskStatus::Running);

        manager.pause(handle.id()).unwrap();
        assert_eq!(handle.status(), TaskStatus::Paused);

        manager.resume(handle.id()).unwrap();
        assert_eq!(handle.status(), TaskStatus::Running);
    }

    #[test]
    fn test_remove_task() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(TaskBuilder::new("Task", "test"));
        let id = handle.id().to_string();

        // Cannot remove active task
        assert!(manager.remove(&id).is_err());

        handle.complete(serde_json::json!({}));

        // Can remove completed task
        assert!(manager.remove(&id).is_ok());
        assert!(manager.get(&id).is_none());
    }

    #[test]
    fn test_task_count() {
        let manager = TaskManager::new(Default::default());

        assert_eq!(manager.task_count(), 0);
        assert_eq!(manager.active_task_count(), 0);

        let h1 = manager.create(TaskBuilder::new("Task 1", "test"));
        let h2 = manager.create(TaskBuilder::new("Task 2", "test"));

        assert_eq!(manager.task_count(), 2);
        assert_eq!(manager.active_task_count(), 2);

        h1.start();
        h1.complete(serde_json::json!({}));

        assert_eq!(manager.task_count(), 2);
        assert_eq!(manager.active_task_count(), 1);

        let _ = h2;
    }

    #[test]
    fn test_cancellation_token() {
        let token = CancellationToken::new();

        assert!(!token.is_cancelled());

        let child = token.child();
        assert!(!child.is_cancelled());

        token.cancel();

        assert!(token.is_cancelled());
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_task_info_serialization() {
        let manager = TaskManager::new(Default::default());
        let handle = manager.create(
            TaskBuilder::new("Task", "test")
                .label("env", "prod")
                .metadata("key", serde_json::json!("value")),
        );

        let info = handle.info();
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: TaskInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, info.id);
        assert_eq!(deserialized.name, info.name);
        assert_eq!(deserialized.status, info.status);
    }
}
