//! Python bindings for ChannelMetrics

use crate::metrics::{ChannelMetrics, MetricsSnapshot};
use pyo3::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Python wrapper for ChannelMetrics.
#[pyclass(name = "ChannelMetrics")]
pub struct PyChannelMetrics {
    inner: Arc<ChannelMetrics>,
}

#[pymethods]
impl PyChannelMetrics {
    /// Create a new metrics instance.
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(ChannelMetrics::new()),
        }
    }

    /// Record a message sent.
    fn record_send(&self, bytes: usize) {
        self.inner.record_send(bytes);
    }

    /// Record a message received.
    fn record_recv(&self, bytes: usize) {
        self.inner.record_recv(bytes);
    }

    /// Record a send error.
    fn record_send_error(&self) {
        self.inner.record_send_error();
    }

    /// Record a receive error.
    fn record_recv_error(&self) {
        self.inner.record_recv_error();
    }

    /// Record latency for a message in microseconds.
    fn record_latency_us(&self, latency_us: u64) {
        self.inner.record_latency(Duration::from_micros(latency_us));
    }

    /// Record latency for a message in milliseconds.
    fn record_latency_ms(&self, latency_ms: u64) {
        self.inner.record_latency(Duration::from_millis(latency_ms));
    }

    /// Update queue depth.
    fn set_queue_depth(&self, depth: u64) {
        self.inner.set_queue_depth(depth);
    }

    /// Get messages sent count.
    #[getter]
    fn messages_sent(&self) -> u64 {
        self.inner.messages_sent()
    }

    /// Get messages received count.
    #[getter]
    fn messages_received(&self) -> u64 {
        self.inner.messages_received()
    }

    /// Get bytes sent count.
    #[getter]
    fn bytes_sent(&self) -> u64 {
        self.inner.bytes_sent()
    }

    /// Get bytes received count.
    #[getter]
    fn bytes_received(&self) -> u64 {
        self.inner.bytes_received()
    }

    /// Get send errors count.
    #[getter]
    fn send_errors(&self) -> u64 {
        self.inner.send_errors()
    }

    /// Get receive errors count.
    #[getter]
    fn receive_errors(&self) -> u64 {
        self.inner.receive_errors()
    }

    /// Get current queue depth.
    #[getter]
    fn queue_depth(&self) -> u64 {
        self.inner.queue_depth()
    }

    /// Get peak queue depth.
    #[getter]
    fn peak_queue_depth(&self) -> u64 {
        self.inner.peak_queue_depth()
    }

    /// Get average latency in microseconds.
    #[getter]
    fn avg_latency_us(&self) -> u64 {
        self.inner.avg_latency_us()
    }

    /// Get minimum latency in microseconds.
    #[getter]
    fn min_latency_us(&self) -> Option<u64> {
        self.inner.min_latency_us()
    }

    /// Get maximum latency in microseconds.
    #[getter]
    fn max_latency_us(&self) -> u64 {
        self.inner.max_latency_us()
    }

    /// Get latency percentile (e.g., 99 for p99).
    fn latency_percentile(&self, percentile: u8) -> u64 {
        self.inner.latency_percentile(percentile)
    }

    /// Get elapsed time in seconds.
    #[getter]
    fn elapsed_secs(&self) -> f64 {
        self.inner.elapsed().as_secs_f64()
    }

    /// Get send throughput in messages per second.
    #[getter]
    fn send_throughput(&self) -> f64 {
        self.inner.send_throughput()
    }

    /// Get receive throughput in messages per second.
    #[getter]
    fn recv_throughput(&self) -> f64 {
        self.inner.recv_throughput()
    }

    /// Get send bandwidth in bytes per second.
    #[getter]
    fn send_bandwidth(&self) -> f64 {
        self.inner.send_bandwidth()
    }

    /// Get receive bandwidth in bytes per second.
    #[getter]
    fn recv_bandwidth(&self) -> f64 {
        self.inner.recv_bandwidth()
    }

    /// Reset all metrics.
    fn reset(&self) {
        self.inner.reset();
    }

    /// Get a snapshot of all metrics as a dict.
    fn snapshot(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let snapshot = self.inner.snapshot();
        snapshot_to_dict(py, &snapshot)
    }

    /// Export metrics as JSON string.
    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    /// Export metrics as pretty JSON string.
    fn to_json_pretty(&self) -> String {
        self.inner.to_json_pretty()
    }

    /// Export metrics in Prometheus format.
    fn to_prometheus(&self, prefix: &str) -> String {
        self.inner.to_prometheus(prefix)
    }

    fn __repr__(&self) -> String {
        format!(
            "ChannelMetrics(sent={}, recv={}, bytes_sent={}, bytes_recv={}, avg_latency={}µs)",
            self.inner.messages_sent(),
            self.inner.messages_received(),
            self.inner.bytes_sent(),
            self.inner.bytes_received(),
            self.inner.avg_latency_us()
        )
    }
}

/// Python wrapper for MetricsSnapshot.
#[pyclass(name = "MetricsSnapshot")]
#[derive(Clone)]
pub struct PyMetricsSnapshot {
    inner: MetricsSnapshot,
}

#[pymethods]
impl PyMetricsSnapshot {
    #[getter]
    fn messages_sent(&self) -> u64 {
        self.inner.messages_sent
    }

    #[getter]
    fn messages_received(&self) -> u64 {
        self.inner.messages_received
    }

    #[getter]
    fn bytes_sent(&self) -> u64 {
        self.inner.bytes_sent
    }

    #[getter]
    fn bytes_received(&self) -> u64 {
        self.inner.bytes_received
    }

    #[getter]
    fn send_errors(&self) -> u64 {
        self.inner.send_errors
    }

    #[getter]
    fn receive_errors(&self) -> u64 {
        self.inner.receive_errors
    }

    #[getter]
    fn queue_depth(&self) -> u64 {
        self.inner.queue_depth
    }

    #[getter]
    fn peak_queue_depth(&self) -> u64 {
        self.inner.peak_queue_depth
    }

    #[getter]
    fn avg_latency_us(&self) -> u64 {
        self.inner.avg_latency_us
    }

    #[getter]
    fn min_latency_us(&self) -> Option<u64> {
        self.inner.min_latency_us
    }

    #[getter]
    fn max_latency_us(&self) -> u64 {
        self.inner.max_latency_us
    }

    #[getter]
    fn p50_latency_us(&self) -> u64 {
        self.inner.p50_latency_us
    }

    #[getter]
    fn p95_latency_us(&self) -> u64 {
        self.inner.p95_latency_us
    }

    #[getter]
    fn p99_latency_us(&self) -> u64 {
        self.inner.p99_latency_us
    }

    #[getter]
    fn elapsed_secs(&self) -> f64 {
        self.inner.elapsed_secs
    }

    #[getter]
    fn send_throughput(&self) -> f64 {
        self.inner.send_throughput
    }

    #[getter]
    fn recv_throughput(&self) -> f64 {
        self.inner.recv_throughput
    }

    #[getter]
    fn send_bandwidth(&self) -> f64 {
        self.inner.send_bandwidth
    }

    #[getter]
    fn recv_bandwidth(&self) -> f64 {
        self.inner.recv_bandwidth
    }

    /// Convert to dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        snapshot_to_dict(py, &self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "MetricsSnapshot(sent={}, recv={}, avg_latency={}µs, p99={}µs)",
            self.inner.messages_sent,
            self.inner.messages_received,
            self.inner.avg_latency_us,
            self.inner.p99_latency_us
        )
    }
}

fn snapshot_to_dict(py: Python<'_>, snapshot: &MetricsSnapshot) -> PyResult<Py<PyAny>> {
    use pyo3::types::PyDict;

    let dict = PyDict::new(py);
    dict.set_item("messages_sent", snapshot.messages_sent)?;
    dict.set_item("messages_received", snapshot.messages_received)?;
    dict.set_item("bytes_sent", snapshot.bytes_sent)?;
    dict.set_item("bytes_received", snapshot.bytes_received)?;
    dict.set_item("send_errors", snapshot.send_errors)?;
    dict.set_item("receive_errors", snapshot.receive_errors)?;
    dict.set_item("queue_depth", snapshot.queue_depth)?;
    dict.set_item("peak_queue_depth", snapshot.peak_queue_depth)?;
    dict.set_item("avg_latency_us", snapshot.avg_latency_us)?;
    dict.set_item("min_latency_us", snapshot.min_latency_us)?;
    dict.set_item("max_latency_us", snapshot.max_latency_us)?;
    dict.set_item("p50_latency_us", snapshot.p50_latency_us)?;
    dict.set_item("p95_latency_us", snapshot.p95_latency_us)?;
    dict.set_item("p99_latency_us", snapshot.p99_latency_us)?;
    dict.set_item("elapsed_secs", snapshot.elapsed_secs)?;
    dict.set_item("send_throughput", snapshot.send_throughput)?;
    dict.set_item("recv_throughput", snapshot.recv_throughput)?;
    dict.set_item("send_bandwidth", snapshot.send_bandwidth)?;
    dict.set_item("recv_bandwidth", snapshot.recv_bandwidth)?;

    Ok(dict.into())
}
