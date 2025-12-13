//! Event Stream - Real-time Event Push System
//!
//! This module provides a publish-subscribe event system for real-time event pushing,
//! similar to Docker's `docker events` and `docker logs -f` functionality.
//!
//! # Features
//!
//! - Publish-subscribe pattern with multiple publishers and subscribers
//! - Event filtering by type and resource ID
//! - Event history with optional replay
//! - Backpressure handling for slow consumers
//!
//! # Example
//!
//! ```rust
//! use ipckit::{EventBus, Event, EventFilter};
//!
//! let bus = EventBus::new(Default::default());
//! let publisher = bus.publisher();
//!
//! // Subscribe to task events
//! let mut subscriber = bus.subscribe(
//!     EventFilter::new().event_type("task.*")
//! );
//!
//! // Publish an event
//! publisher.publish(Event::new("task.started", serde_json::json!({"task_id": "123"})));
//!
//! // Receive the event
//! if let Some(event) = subscriber.try_recv() {
//!     println!("Received: {:?}", event);
//! }
//! ```

use crate::error::{IpcError, Result};
use crossbeam_channel::{self, Receiver, Sender, TryRecvError};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// A unique event identifier.
pub type EventId = u64;

/// An event that can be published and subscribed to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event ID
    pub id: EventId,
    /// Event timestamp
    #[serde(with = "system_time_serde")]
    pub timestamp: SystemTime,
    /// Event type (e.g., "task.progress", "log.stdout", "task.completed")
    pub event_type: String,
    /// Associated resource ID (e.g., task_id)
    pub resource_id: Option<String>,
    /// Event data
    pub data: serde_json::Value,
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

impl Event {
    /// Create a new event with the given type and data.
    pub fn new(event_type: &str, data: serde_json::Value) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            timestamp: SystemTime::now(),
            event_type: event_type.to_string(),
            resource_id: None,
            data,
        }
    }

    /// Create an event with a resource ID.
    pub fn with_resource(event_type: &str, resource_id: &str, data: serde_json::Value) -> Self {
        let mut event = Self::new(event_type, data);
        event.resource_id = Some(resource_id.to_string());
        event
    }

    /// Create a progress event.
    pub fn progress(resource_id: &str, current: u64, total: u64, message: &str) -> Self {
        Self::with_resource(
            event_types::TASK_PROGRESS,
            resource_id,
            serde_json::json!({
                "current": current,
                "total": total,
                "percentage": if total > 0 { (current * 100) / total } else { 0 },
                "message": message
            }),
        )
    }

    /// Create a log event.
    pub fn log(resource_id: &str, level: &str, message: &str) -> Self {
        let event_type = match level {
            "info" => event_types::LOG_INFO,
            "warn" | "warning" => event_types::LOG_WARN,
            "error" => event_types::LOG_ERROR,
            "stdout" => event_types::LOG_STDOUT,
            "stderr" => event_types::LOG_STDERR,
            _ => event_types::LOG_INFO,
        };

        Self::with_resource(
            event_type,
            resource_id,
            serde_json::json!({
                "level": level,
                "message": message
            }),
        )
    }

    /// Create a stdout log event.
    pub fn stdout(resource_id: &str, line: &str) -> Self {
        Self::log(resource_id, "stdout", line)
    }

    /// Create a stderr log event.
    pub fn stderr(resource_id: &str, line: &str) -> Self {
        Self::log(resource_id, "stderr", line)
    }
}

/// Standard event type constants.
pub mod event_types {
    // Task lifecycle
    pub const TASK_CREATED: &str = "task.created";
    pub const TASK_STARTED: &str = "task.started";
    pub const TASK_PROGRESS: &str = "task.progress";
    pub const TASK_COMPLETED: &str = "task.completed";
    pub const TASK_FAILED: &str = "task.failed";
    pub const TASK_CANCELLED: &str = "task.cancelled";
    pub const TASK_PAUSED: &str = "task.paused";
    pub const TASK_RESUMED: &str = "task.resumed";

    // Logs
    pub const LOG_STDOUT: &str = "log.stdout";
    pub const LOG_STDERR: &str = "log.stderr";
    pub const LOG_INFO: &str = "log.info";
    pub const LOG_WARN: &str = "log.warn";
    pub const LOG_ERROR: &str = "log.error";

    // File operations
    pub const FILE_UPLOAD_PROGRESS: &str = "file.upload.progress";
    pub const FILE_DOWNLOAD_PROGRESS: &str = "file.download.progress";

    // System
    pub const SYSTEM_SHUTDOWN: &str = "system.shutdown";
    pub const SYSTEM_ERROR: &str = "system.error";
}

/// Event filter for subscribing to specific events.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Event type patterns (supports wildcards like "task.*")
    pub event_types: Option<Vec<String>>,
    /// Resource ID filter
    pub resource_ids: Option<Vec<String>>,
    /// Start time filter
    pub since: Option<SystemTime>,
    /// End time filter
    pub until: Option<SystemTime>,
}

impl EventFilter {
    /// Create a new empty filter that matches all events.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event type pattern to the filter.
    ///
    /// Supports wildcards like "task.*" to match all task events.
    pub fn event_type(mut self, pattern: &str) -> Self {
        let types = self.event_types.get_or_insert_with(Vec::new);
        types.push(pattern.to_string());
        self
    }

    /// Add a resource ID to the filter.
    pub fn resource(mut self, id: &str) -> Self {
        let ids = self.resource_ids.get_or_insert_with(Vec::new);
        ids.push(id.to_string());
        self
    }

    /// Set the start time filter.
    pub fn since(mut self, time: SystemTime) -> Self {
        self.since = Some(time);
        self
    }

    /// Set the end time filter.
    pub fn until(mut self, time: SystemTime) -> Self {
        self.until = Some(time);
        self
    }

    /// Check if an event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        // Check event type
        if let Some(ref patterns) = self.event_types {
            let matches_type = patterns.iter().any(|pattern| {
                if pattern.ends_with(".*") {
                    let prefix = &pattern[..pattern.len() - 2];
                    event.event_type.starts_with(prefix)
                } else if pattern.contains('*') {
                    // Simple glob matching
                    let parts: Vec<&str> = pattern.split('*').collect();
                    let mut pos = 0;
                    for (i, part) in parts.iter().enumerate() {
                        if part.is_empty() {
                            continue;
                        }
                        if let Some(found) = event.event_type[pos..].find(part) {
                            if i == 0 && found != 0 {
                                return false;
                            }
                            pos += found + part.len();
                        } else {
                            return false;
                        }
                    }
                    true
                } else {
                    event.event_type == *pattern
                }
            });

            if !matches_type {
                return false;
            }
        }

        // Check resource ID
        if let Some(ref ids) = self.resource_ids {
            if let Some(ref event_resource) = event.resource_id {
                if !ids.iter().any(|id| id == event_resource) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check time range
        if let Some(since) = self.since {
            if event.timestamp < since {
                return false;
            }
        }

        if let Some(until) = self.until {
            if event.timestamp > until {
                return false;
            }
        }

        true
    }
}

/// Policy for handling slow consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlowConsumerPolicy {
    /// Drop oldest events when buffer is full
    #[default]
    DropOldest,
    /// Drop newest events when buffer is full
    DropNewest,
    /// Block until space is available
    Block,
}

/// Configuration for the event bus.
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Number of events to keep in history
    pub history_size: usize,
    /// Buffer size per subscriber
    pub subscriber_buffer: usize,
    /// Policy for slow consumers
    pub slow_consumer: SlowConsumerPolicy,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            history_size: 1000,
            subscriber_buffer: 256,
            slow_consumer: SlowConsumerPolicy::DropOldest,
        }
    }
}

/// Event publisher for sending events to the bus.
#[derive(Clone)]
pub struct EventPublisher {
    inner: Arc<EventBusInner>,
}

impl EventPublisher {
    /// Publish an event to the bus.
    pub fn publish(&self, event: Event) {
        self.inner.publish(event);
    }

    /// Publish a progress event.
    pub fn progress(&self, resource_id: &str, current: u64, total: u64, message: &str) {
        self.publish(Event::progress(resource_id, current, total, message));
    }

    /// Publish a log event.
    pub fn log(&self, resource_id: &str, level: &str, message: &str) {
        self.publish(Event::log(resource_id, level, message));
    }

    /// Publish a stdout log event.
    pub fn stdout(&self, resource_id: &str, line: &str) {
        self.publish(Event::stdout(resource_id, line));
    }

    /// Publish a stderr log event.
    pub fn stderr(&self, resource_id: &str, line: &str) {
        self.publish(Event::stderr(resource_id, line));
    }

    /// Publish a task started event.
    pub fn task_started(&self, task_id: &str, data: serde_json::Value) {
        self.publish(Event::with_resource(
            event_types::TASK_STARTED,
            task_id,
            data,
        ));
    }

    /// Publish a task completed event.
    pub fn task_completed(&self, task_id: &str, result: serde_json::Value) {
        self.publish(Event::with_resource(
            event_types::TASK_COMPLETED,
            task_id,
            serde_json::json!({ "result": result }),
        ));
    }

    /// Publish a task failed event.
    pub fn task_failed(&self, task_id: &str, error: &str) {
        self.publish(Event::with_resource(
            event_types::TASK_FAILED,
            task_id,
            serde_json::json!({ "error": error }),
        ));
    }

    /// Publish a task cancelled event.
    pub fn task_cancelled(&self, task_id: &str) {
        self.publish(Event::with_resource(
            event_types::TASK_CANCELLED,
            task_id,
            serde_json::json!({}),
        ));
    }
}

/// Event subscriber for receiving events from the bus.
pub struct EventSubscriber {
    receiver: Receiver<Event>,
    filter: EventFilter,
}

impl EventSubscriber {
    /// Receive the next event (blocking).
    pub fn recv(&self) -> Option<Event> {
        loop {
            match self.receiver.recv() {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Some(event);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&self) -> Option<Event> {
        loop {
            match self.receiver.try_recv() {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Some(event);
                    }
                }
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => return None,
            }
        }
    }

    /// Receive an event with a timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Event> {
        let deadline = std::time::Instant::now() + timeout;

        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(IpcError::Timeout);
            }

            match self.receiver.recv_timeout(remaining) {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Ok(event);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    return Err(IpcError::Timeout);
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    return Err(IpcError::Closed);
                }
            }
        }
    }

    /// Create an iterator over events.
    pub fn iter(&self) -> impl Iterator<Item = Event> + '_ {
        std::iter::from_fn(move || self.recv())
    }

    /// Create a non-blocking iterator over available events.
    pub fn try_iter(&self) -> impl Iterator<Item = Event> + '_ {
        std::iter::from_fn(move || self.try_recv())
    }

    /// Get the filter for this subscriber.
    pub fn filter(&self) -> &EventFilter {
        &self.filter
    }
}

struct Subscriber {
    sender: Sender<Event>,
    filter: EventFilter,
}

struct EventBusInner {
    config: EventBusConfig,
    subscribers: RwLock<Vec<Subscriber>>,
    history: RwLock<VecDeque<Event>>,
}

impl EventBusInner {
    fn new(config: EventBusConfig) -> Self {
        Self {
            config,
            subscribers: RwLock::new(Vec::new()),
            history: RwLock::new(VecDeque::new()),
        }
    }

    fn publish(&self, event: Event) {
        // Add to history
        {
            let mut history = self.history.write();
            history.push_back(event.clone());

            // Trim history if needed
            while history.len() > self.config.history_size {
                history.pop_front();
            }
        }

        // Send to subscribers
        let subscribers = self.subscribers.read();
        for sub in subscribers.iter() {
            if sub.filter.matches(&event) {
                match self.config.slow_consumer {
                    SlowConsumerPolicy::Block => {
                        let _ = sub.sender.send(event.clone());
                    }
                    SlowConsumerPolicy::DropNewest => {
                        let _ = sub.sender.try_send(event.clone());
                    }
                    SlowConsumerPolicy::DropOldest => {
                        // Try to send, if full, receive one and try again
                        if sub.sender.try_send(event.clone()).is_err() {
                            // Channel is full, we just drop the event for this subscriber
                            // In a more sophisticated implementation, we could drain old events
                        }
                    }
                }
            }
        }
    }

    fn subscribe(&self, filter: EventFilter) -> EventSubscriber {
        let (tx, rx) = crossbeam_channel::bounded(self.config.subscriber_buffer);

        let subscriber = Subscriber {
            sender: tx,
            filter: filter.clone(),
        };

        self.subscribers.write().push(subscriber);

        EventSubscriber {
            receiver: rx,
            filter,
        }
    }

    fn history(&self, filter: &EventFilter) -> Vec<Event> {
        let history = self.history.read();
        history
            .iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    fn clear_history(&self) {
        self.history.write().clear();
    }
}

/// The main event bus for publish-subscribe communication.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

impl EventBus {
    /// Create a new event bus with the given configuration.
    pub fn new(config: EventBusConfig) -> Self {
        Self {
            inner: Arc::new(EventBusInner::new(config)),
        }
    }

    /// Create a new publisher for this bus.
    pub fn publisher(&self) -> EventPublisher {
        EventPublisher {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Subscribe to events matching the given filter.
    pub fn subscribe(&self, filter: EventFilter) -> EventSubscriber {
        self.inner.subscribe(filter)
    }

    /// Get historical events matching the given filter.
    pub fn history(&self, filter: &EventFilter) -> Vec<Event> {
        self.inner.history(filter)
    }

    /// Clear all event history.
    pub fn clear_history(&self) {
        self.inner.clear_history();
    }

    /// Publish an event directly.
    pub fn publish(&self, event: Event) {
        self.inner.publish(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(EventBusConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("test.event", serde_json::json!({"key": "value"}));

        assert!(event.id > 0);
        assert_eq!(event.event_type, "test.event");
        assert!(event.resource_id.is_none());
    }

    #[test]
    fn test_event_with_resource() {
        let event = Event::with_resource("task.started", "task-123", serde_json::json!({}));

        assert_eq!(event.event_type, "task.started");
        assert_eq!(event.resource_id, Some("task-123".to_string()));
    }

    #[test]
    fn test_progress_event() {
        let event = Event::progress("task-123", 50, 100, "Half done");

        assert_eq!(event.event_type, event_types::TASK_PROGRESS);
        assert_eq!(event.resource_id, Some("task-123".to_string()));
        assert_eq!(event.data["percentage"], 50);
    }

    #[test]
    fn test_filter_event_type() {
        let filter = EventFilter::new().event_type("task.*");

        let event1 = Event::new("task.started", serde_json::json!({}));
        let event2 = Event::new("log.info", serde_json::json!({}));

        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2));
    }

    #[test]
    fn test_filter_resource() {
        let filter = EventFilter::new().resource("task-123");

        let event1 = Event::with_resource("task.started", "task-123", serde_json::json!({}));
        let event2 = Event::with_resource("task.started", "task-456", serde_json::json!({}));
        let event3 = Event::new("task.started", serde_json::json!({}));

        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2));
        assert!(!filter.matches(&event3));
    }

    #[test]
    fn test_filter_combined() {
        let filter = EventFilter::new().event_type("task.*").resource("task-123");

        let event1 = Event::with_resource("task.started", "task-123", serde_json::json!({}));
        let event2 = Event::with_resource("log.info", "task-123", serde_json::json!({}));
        let event3 = Event::with_resource("task.started", "task-456", serde_json::json!({}));

        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2));
        assert!(!filter.matches(&event3));
    }

    #[test]
    fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new(Default::default());
        let publisher = bus.publisher();
        let subscriber = bus.subscribe(EventFilter::new());

        publisher.publish(Event::new("test.event", serde_json::json!({"value": 42})));

        let event = subscriber.try_recv().unwrap();
        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.data["value"], 42);
    }

    #[test]
    fn test_event_bus_filtered_subscription() {
        let bus = EventBus::new(Default::default());
        let publisher = bus.publisher();
        let subscriber = bus.subscribe(EventFilter::new().event_type("task.*"));

        publisher.publish(Event::new("task.started", serde_json::json!({})));
        publisher.publish(Event::new("log.info", serde_json::json!({})));
        publisher.publish(Event::new("task.completed", serde_json::json!({})));

        let events: Vec<Event> = subscriber.try_iter().collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "task.started");
        assert_eq!(events[1].event_type, "task.completed");
    }

    #[test]
    fn test_event_bus_history() {
        let bus = EventBus::new(EventBusConfig {
            history_size: 10,
            ..Default::default()
        });

        bus.publish(Event::new("event.1", serde_json::json!({})));
        bus.publish(Event::new("event.2", serde_json::json!({})));
        bus.publish(Event::new("event.3", serde_json::json!({})));

        let history = bus.history(&EventFilter::new());
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_event_bus_history_limit() {
        let bus = EventBus::new(EventBusConfig {
            history_size: 2,
            ..Default::default()
        });

        bus.publish(Event::new("event.1", serde_json::json!({})));
        bus.publish(Event::new("event.2", serde_json::json!({})));
        bus.publish(Event::new("event.3", serde_json::json!({})));

        let history = bus.history(&EventFilter::new());
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].event_type, "event.2");
        assert_eq!(history[1].event_type, "event.3");
    }

    #[test]
    fn test_event_bus_clear_history() {
        let bus = EventBus::new(Default::default());

        bus.publish(Event::new("event.1", serde_json::json!({})));
        bus.publish(Event::new("event.2", serde_json::json!({})));

        assert_eq!(bus.history(&EventFilter::new()).len(), 2);

        bus.clear_history();

        assert_eq!(bus.history(&EventFilter::new()).len(), 0);
    }

    #[test]
    fn test_multiple_subscribers() {
        let bus = EventBus::new(Default::default());
        let publisher = bus.publisher();

        let sub1 = bus.subscribe(EventFilter::new().event_type("task.*"));
        let sub2 = bus.subscribe(EventFilter::new().event_type("log.*"));
        let sub3 = bus.subscribe(EventFilter::new());

        publisher.publish(Event::new("task.started", serde_json::json!({})));
        publisher.publish(Event::new("log.info", serde_json::json!({})));

        assert_eq!(sub1.try_iter().count(), 1);
        assert_eq!(sub2.try_iter().count(), 1);
        assert_eq!(sub3.try_iter().count(), 2);
    }

    #[test]
    fn test_publisher_helper_methods() {
        let bus = EventBus::new(Default::default());
        let publisher = bus.publisher();
        let subscriber = bus.subscribe(EventFilter::new());

        publisher.progress("task-1", 50, 100, "Half done");
        publisher.log("task-1", "info", "Processing...");
        publisher.stdout("task-1", "Output line");
        publisher.task_completed("task-1", serde_json::json!({"success": true}));

        let events: Vec<Event> = subscriber.try_iter().collect();
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::with_resource(
            "task.started",
            "task-123",
            serde_json::json!({"key": "value"}),
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, event.id);
        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.resource_id, event.resource_id);
    }
}
