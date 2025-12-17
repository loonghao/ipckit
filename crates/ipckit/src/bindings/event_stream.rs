//! Python bindings for EventStream (Event Bus)

use crate::bindings::json_utils::{json_value_to_py, py_to_json_value};
use crate::event_stream::{
    Event, EventBus, EventBusConfig, EventFilter, EventPublisher, EventSubscriber,
    SlowConsumerPolicy,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::time::{Duration, UNIX_EPOCH};

/// Python wrapper for Event.
#[pyclass(name = "Event")]
#[derive(Clone)]
pub struct PyEvent {
    inner: Event,
}

#[pymethods]
impl PyEvent {
    /// Create a new event with the given type and data.
    #[new]
    #[pyo3(signature = (event_type, data=None))]
    fn new(py: Python<'_>, event_type: &str, data: Option<Py<PyAny>>) -> PyResult<Self> {
        let json_data = match data {
            Some(obj) => py_to_json_value(&obj.bind(py).clone())?,
            None => serde_json::json!({}),
        };
        Ok(Self {
            inner: Event::new(event_type, json_data),
        })
    }

    /// Create an event with a resource ID.
    #[staticmethod]
    #[pyo3(signature = (event_type, resource_id, data=None))]
    fn with_resource(
        py: Python<'_>,
        event_type: &str,
        resource_id: &str,
        data: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        let json_data = match data {
            Some(obj) => py_to_json_value(&obj.bind(py).clone())?,
            None => serde_json::json!({}),
        };
        Ok(Self {
            inner: Event::with_resource(event_type, resource_id, json_data),
        })
    }

    /// Create a progress event.
    #[staticmethod]
    fn progress(resource_id: &str, current: u64, total: u64, message: &str) -> Self {
        Self {
            inner: Event::progress(resource_id, current, total, message),
        }
    }

    /// Create a log event.
    #[staticmethod]
    fn log(resource_id: &str, level: &str, message: &str) -> Self {
        Self {
            inner: Event::log(resource_id, level, message),
        }
    }

    /// Create a stdout log event.
    #[staticmethod]
    fn stdout(resource_id: &str, line: &str) -> Self {
        Self {
            inner: Event::stdout(resource_id, line),
        }
    }

    /// Create a stderr log event.
    #[staticmethod]
    fn stderr(resource_id: &str, line: &str) -> Self {
        Self {
            inner: Event::stderr(resource_id, line),
        }
    }

    /// Get the event ID.
    #[getter]
    fn id(&self) -> u64 {
        self.inner.id
    }

    /// Get the event timestamp as Unix timestamp (seconds).
    #[getter]
    fn timestamp(&self) -> f64 {
        self.inner
            .timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64()
    }

    /// Get the event type.
    #[getter]
    fn event_type(&self) -> &str {
        &self.inner.event_type
    }

    /// Get the resource ID.
    #[getter]
    fn resource_id(&self) -> Option<&str> {
        self.inner.resource_id.as_deref()
    }

    /// Get the event data as a Python object.
    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_value_to_py(py, &self.inner.data)
    }

    /// Convert to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Event(id={}, type='{}', resource_id={:?})",
            self.inner.id, self.inner.event_type, self.inner.resource_id
        )
    }
}

/// Python wrapper for EventFilter.
#[pyclass(name = "EventFilter")]
#[derive(Clone)]
pub struct PyEventFilter {
    inner: EventFilter,
}

#[pymethods]
impl PyEventFilter {
    /// Create a new empty filter that matches all events.
    #[new]
    fn new() -> Self {
        Self {
            inner: EventFilter::new(),
        }
    }

    /// Add an event type pattern to the filter.
    /// Supports wildcards like "task.*" to match all task events.
    fn event_type(&self, pattern: &str) -> Self {
        Self {
            inner: self.inner.clone().event_type(pattern),
        }
    }

    /// Add a resource ID to the filter.
    fn resource(&self, id: &str) -> Self {
        Self {
            inner: self.inner.clone().resource(id),
        }
    }

    /// Set the start time filter (Unix timestamp in seconds).
    fn since(&self, timestamp: f64) -> Self {
        let time = UNIX_EPOCH + Duration::from_secs_f64(timestamp);
        Self {
            inner: self.inner.clone().since(time),
        }
    }

    /// Set the end time filter (Unix timestamp in seconds).
    fn until(&self, timestamp: f64) -> Self {
        let time = UNIX_EPOCH + Duration::from_secs_f64(timestamp);
        Self {
            inner: self.inner.clone().until(time),
        }
    }

    /// Check if an event matches this filter.
    fn matches(&self, event: &PyEvent) -> bool {
        self.inner.matches(&event.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "EventFilter(event_types={:?}, resource_ids={:?})",
            self.inner.event_types, self.inner.resource_ids
        )
    }
}

/// Python wrapper for EventBusConfig.
#[pyclass(name = "EventBusConfig")]
#[derive(Clone)]
pub struct PyEventBusConfig {
    inner: EventBusConfig,
}

#[pymethods]
impl PyEventBusConfig {
    /// Create a new event bus configuration.
    #[new]
    #[pyo3(signature = (history_size=1000, subscriber_buffer=256, slow_consumer="drop_oldest"))]
    fn new(history_size: usize, subscriber_buffer: usize, slow_consumer: &str) -> PyResult<Self> {
        let policy =
            match slow_consumer {
                "drop_oldest" => SlowConsumerPolicy::DropOldest,
                "drop_newest" => SlowConsumerPolicy::DropNewest,
                "block" => SlowConsumerPolicy::Block,
                _ => return Err(PyRuntimeError::new_err(
                    "Invalid slow_consumer policy. Use 'drop_oldest', 'drop_newest', or 'block'",
                )),
            };

        Ok(Self {
            inner: EventBusConfig {
                history_size,
                subscriber_buffer,
                slow_consumer: policy,
            },
        })
    }

    /// Get the history size.
    #[getter]
    fn history_size(&self) -> usize {
        self.inner.history_size
    }

    /// Get the subscriber buffer size.
    #[getter]
    fn subscriber_buffer(&self) -> usize {
        self.inner.subscriber_buffer
    }

    fn __repr__(&self) -> String {
        format!(
            "EventBusConfig(history_size={}, subscriber_buffer={})",
            self.inner.history_size, self.inner.subscriber_buffer
        )
    }
}

/// Python wrapper for EventPublisher.
#[pyclass(name = "EventPublisher")]
pub struct PyEventPublisher {
    inner: EventPublisher,
}

#[pymethods]
impl PyEventPublisher {
    /// Publish an event to the bus.
    fn publish(&self, event: &PyEvent) {
        self.inner.publish(event.inner.clone());
    }

    /// Publish a progress event.
    fn progress(&self, resource_id: &str, current: u64, total: u64, message: &str) {
        self.inner.progress(resource_id, current, total, message);
    }

    /// Publish a log event.
    fn log(&self, resource_id: &str, level: &str, message: &str) {
        self.inner.log(resource_id, level, message);
    }

    /// Publish a stdout log event.
    fn stdout(&self, resource_id: &str, line: &str) {
        self.inner.stdout(resource_id, line);
    }

    /// Publish a stderr log event.
    fn stderr(&self, resource_id: &str, line: &str) {
        self.inner.stderr(resource_id, line);
    }

    /// Publish a task started event.
    fn task_started(&self, py: Python<'_>, task_id: &str, data: Option<Py<PyAny>>) -> PyResult<()> {
        let json_data = match data {
            Some(obj) => py_to_json_value(&obj.bind(py).clone())?,
            None => serde_json::json!({}),
        };
        self.inner.task_started(task_id, json_data);
        Ok(())
    }

    /// Publish a task completed event.
    fn task_completed(
        &self,
        py: Python<'_>,
        task_id: &str,
        result: Option<Py<PyAny>>,
    ) -> PyResult<()> {
        let json_result = match result {
            Some(obj) => py_to_json_value(&obj.bind(py).clone())?,
            None => serde_json::json!({}),
        };
        self.inner.task_completed(task_id, json_result);
        Ok(())
    }

    /// Publish a task failed event.
    fn task_failed(&self, task_id: &str, error: &str) {
        self.inner.task_failed(task_id, error);
    }

    /// Publish a task cancelled event.
    fn task_cancelled(&self, task_id: &str) {
        self.inner.task_cancelled(task_id);
    }

    fn __repr__(&self) -> String {
        "EventPublisher()".to_string()
    }
}

/// Python wrapper for EventSubscriber.
#[pyclass(name = "EventSubscriber")]
pub struct PyEventSubscriber {
    inner: EventSubscriber,
}

#[pymethods]
impl PyEventSubscriber {
    /// Receive the next event (blocking).
    /// Returns None if the bus is closed.
    fn recv(&self, py: Python<'_>) -> Option<PyEvent> {
        let inner = &self.inner;
        py.detach(|| inner.recv().map(|e| PyEvent { inner: e }))
    }

    /// Try to receive an event without blocking.
    /// Returns None if no event is available.
    fn try_recv(&self) -> Option<PyEvent> {
        self.inner.try_recv().map(|e| PyEvent { inner: e })
    }

    /// Receive an event with a timeout in milliseconds.
    /// Raises RuntimeError on timeout.
    fn recv_timeout(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<PyEvent> {
        let timeout = Duration::from_millis(timeout_ms);
        let inner = &self.inner;
        py.detach(|| {
            inner
                .recv_timeout(timeout)
                .map(|e| PyEvent { inner: e })
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Get all currently available events without blocking.
    fn drain(&self) -> Vec<PyEvent> {
        self.inner
            .try_iter()
            .map(|e| PyEvent { inner: e })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("EventSubscriber(filter={:?})", self.inner.filter())
    }
}

/// Python wrapper for EventBus.
#[pyclass(name = "EventBus")]
pub struct PyEventBus {
    inner: EventBus,
}

#[pymethods]
impl PyEventBus {
    /// Create a new event bus with optional configuration.
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<PyEventBusConfig>) -> Self {
        let cfg = config.map(|c| c.inner).unwrap_or_default();
        Self {
            inner: EventBus::new(cfg),
        }
    }

    /// Create a new publisher for this bus.
    fn publisher(&self) -> PyEventPublisher {
        PyEventPublisher {
            inner: self.inner.publisher(),
        }
    }

    /// Subscribe to events matching the given filter.
    #[pyo3(signature = (filter=None))]
    fn subscribe(&self, filter: Option<PyEventFilter>) -> PyEventSubscriber {
        let f = filter.map(|f| f.inner).unwrap_or_default();
        PyEventSubscriber {
            inner: self.inner.subscribe(f),
        }
    }

    /// Get historical events matching the given filter.
    #[pyo3(signature = (filter=None))]
    fn history(&self, filter: Option<PyEventFilter>) -> Vec<PyEvent> {
        let f = filter.map(|f| f.inner).unwrap_or_default();
        self.inner
            .history(&f)
            .into_iter()
            .map(|e| PyEvent { inner: e })
            .collect()
    }

    /// Clear all event history.
    fn clear_history(&self) {
        self.inner.clear_history();
    }

    /// Publish an event directly.
    fn publish(&self, event: &PyEvent) {
        self.inner.publish(event.inner.clone());
    }

    fn __repr__(&self) -> String {
        "EventBus()".to_string()
    }
}
