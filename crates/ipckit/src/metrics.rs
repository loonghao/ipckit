//! # Channel Metrics
//!
//! This module provides performance monitoring capabilities for IPC channels.
//! It tracks message counts, byte throughput, errors, latency, and queue depth.
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit::{ChannelMetrics, MeteredChannel};
//!
//! let channel = NamedPipeChannel::new("my_pipe")?.with_metrics();
//!
//! // ... use channel ...
//!
//! let metrics = channel.metrics();
//! println!("Messages sent: {}", metrics.messages_sent());
//! println!("Avg latency: {}µs", metrics.avg_latency_us());
//!
//! // Export for monitoring
//! log::info!("IPC metrics: {}", metrics.to_json());
//! ```

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Atomic metrics counters for thread-safe updates.
#[derive(Debug, Default)]
pub struct ChannelMetrics {
    /// Total messages sent
    messages_sent: AtomicU64,
    /// Total messages received
    messages_received: AtomicU64,
    /// Total bytes sent
    bytes_sent: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Send errors
    send_errors: AtomicU64,
    /// Receive errors
    receive_errors: AtomicU64,
    /// Current queue depth (for buffered channels)
    queue_depth: AtomicU64,
    /// Peak queue depth
    peak_queue_depth: AtomicU64,
    /// Sum of latencies in microseconds (for averaging)
    latency_sum_us: AtomicU64,
    /// Count of latency samples
    latency_count: AtomicU64,
    /// Minimum latency in microseconds
    min_latency_us: AtomicU64,
    /// Maximum latency in microseconds
    max_latency_us: AtomicU64,
    /// Histogram for latency distribution
    latency_histogram: RwLock<LatencyHistogram>,
    /// Start time for rate calculations
    start_time: RwLock<Option<Instant>>,
}

impl ChannelMetrics {
    /// Create a new metrics instance.
    pub fn new() -> Self {
        Self {
            min_latency_us: AtomicU64::new(u64::MAX),
            ..Default::default()
        }
    }

    /// Record a message sent.
    pub fn record_send(&self, bytes: usize) {
        self.ensure_started();
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record a message received.
    pub fn record_recv(&self, bytes: usize) {
        self.ensure_started();
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received
            .fetch_add(bytes as u64, Ordering::Relaxed);
    }

    /// Record a send error.
    pub fn record_send_error(&self) {
        self.send_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a receive error.
    pub fn record_recv_error(&self) {
        self.receive_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record latency for a message.
    pub fn record_latency(&self, latency: Duration) {
        let us = latency.as_micros() as u64;
        self.latency_sum_us.fetch_add(us, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);

        // Update min latency
        let mut current_min = self.min_latency_us.load(Ordering::Relaxed);
        while us < current_min {
            match self.min_latency_us.compare_exchange_weak(
                current_min,
                us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // Update max latency
        let mut current_max = self.max_latency_us.load(Ordering::Relaxed);
        while us > current_max {
            match self.max_latency_us.compare_exchange_weak(
                current_max,
                us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        // Update histogram
        self.latency_histogram.write().record(us);
    }

    /// Update queue depth.
    pub fn set_queue_depth(&self, depth: u64) {
        self.queue_depth.store(depth, Ordering::Relaxed);

        // Update peak
        let mut current_peak = self.peak_queue_depth.load(Ordering::Relaxed);
        while depth > current_peak {
            match self.peak_queue_depth.compare_exchange_weak(
                current_peak,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_peak = x,
            }
        }
    }

    /// Get messages sent count.
    pub fn messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::Relaxed)
    }

    /// Get messages received count.
    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Get bytes sent count.
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get bytes received count.
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Get send errors count.
    pub fn send_errors(&self) -> u64 {
        self.send_errors.load(Ordering::Relaxed)
    }

    /// Get receive errors count.
    pub fn receive_errors(&self) -> u64 {
        self.receive_errors.load(Ordering::Relaxed)
    }

    /// Get current queue depth.
    pub fn queue_depth(&self) -> u64 {
        self.queue_depth.load(Ordering::Relaxed)
    }

    /// Get peak queue depth.
    pub fn peak_queue_depth(&self) -> u64 {
        self.peak_queue_depth.load(Ordering::Relaxed)
    }

    /// Get average latency in microseconds.
    pub fn avg_latency_us(&self) -> u64 {
        let count = self.latency_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.latency_sum_us.load(Ordering::Relaxed) / count
    }

    /// Get minimum latency in microseconds.
    pub fn min_latency_us(&self) -> Option<u64> {
        let min = self.min_latency_us.load(Ordering::Relaxed);
        if min == u64::MAX {
            None
        } else {
            Some(min)
        }
    }

    /// Get maximum latency in microseconds.
    pub fn max_latency_us(&self) -> u64 {
        self.max_latency_us.load(Ordering::Relaxed)
    }

    /// Get latency percentile (e.g., 99 for p99).
    pub fn latency_percentile(&self, percentile: u8) -> u64 {
        self.latency_histogram.read().percentile(percentile)
    }

    /// Get elapsed time since metrics started.
    pub fn elapsed(&self) -> Duration {
        self.start_time
            .read()
            .map(|t| t.elapsed())
            .unwrap_or_default()
    }

    /// Get send throughput in messages per second.
    pub fn send_throughput(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.messages_sent() as f64 / elapsed
    }

    /// Get receive throughput in messages per second.
    pub fn recv_throughput(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.messages_received() as f64 / elapsed
    }

    /// Get send bandwidth in bytes per second.
    pub fn send_bandwidth(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.bytes_sent() as f64 / elapsed
    }

    /// Get receive bandwidth in bytes per second.
    pub fn recv_bandwidth(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.bytes_received() as f64 / elapsed
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.messages_sent.store(0, Ordering::Relaxed);
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.send_errors.store(0, Ordering::Relaxed);
        self.receive_errors.store(0, Ordering::Relaxed);
        self.queue_depth.store(0, Ordering::Relaxed);
        self.peak_queue_depth.store(0, Ordering::Relaxed);
        self.latency_sum_us.store(0, Ordering::Relaxed);
        self.latency_count.store(0, Ordering::Relaxed);
        self.min_latency_us.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_us.store(0, Ordering::Relaxed);
        self.latency_histogram.write().reset();
        *self.start_time.write() = Some(Instant::now());
    }

    /// Get a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_sent: self.messages_sent(),
            messages_received: self.messages_received(),
            bytes_sent: self.bytes_sent(),
            bytes_received: self.bytes_received(),
            send_errors: self.send_errors(),
            receive_errors: self.receive_errors(),
            queue_depth: self.queue_depth(),
            peak_queue_depth: self.peak_queue_depth(),
            avg_latency_us: self.avg_latency_us(),
            min_latency_us: self.min_latency_us(),
            max_latency_us: self.max_latency_us(),
            p50_latency_us: self.latency_percentile(50),
            p95_latency_us: self.latency_percentile(95),
            p99_latency_us: self.latency_percentile(99),
            elapsed_secs: self.elapsed().as_secs_f64(),
            send_throughput: self.send_throughput(),
            recv_throughput: self.recv_throughput(),
            send_bandwidth: self.send_bandwidth(),
            recv_bandwidth: self.recv_bandwidth(),
        }
    }

    /// Export metrics as JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.snapshot()).unwrap_or_default()
    }

    /// Export metrics as pretty JSON string.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.snapshot()).unwrap_or_default()
    }

    /// Export metrics in Prometheus format.
    pub fn to_prometheus(&self, prefix: &str) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        output.push_str(&format!(
            "# HELP {prefix}_messages_sent_total Total messages sent\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_messages_sent_total counter\n"));
        output.push_str(&format!(
            "{prefix}_messages_sent_total {}\n",
            snapshot.messages_sent
        ));

        output.push_str(&format!(
            "# HELP {prefix}_messages_received_total Total messages received\n"
        ));
        output.push_str(&format!(
            "# TYPE {prefix}_messages_received_total counter\n"
        ));
        output.push_str(&format!(
            "{prefix}_messages_received_total {}\n",
            snapshot.messages_received
        ));

        output.push_str(&format!(
            "# HELP {prefix}_bytes_sent_total Total bytes sent\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_bytes_sent_total counter\n"));
        output.push_str(&format!(
            "{prefix}_bytes_sent_total {}\n",
            snapshot.bytes_sent
        ));

        output.push_str(&format!(
            "# HELP {prefix}_bytes_received_total Total bytes received\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_bytes_received_total counter\n"));
        output.push_str(&format!(
            "{prefix}_bytes_received_total {}\n",
            snapshot.bytes_received
        ));

        output.push_str(&format!(
            "# HELP {prefix}_send_errors_total Total send errors\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_send_errors_total counter\n"));
        output.push_str(&format!(
            "{prefix}_send_errors_total {}\n",
            snapshot.send_errors
        ));

        output.push_str(&format!(
            "# HELP {prefix}_receive_errors_total Total receive errors\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_receive_errors_total counter\n"));
        output.push_str(&format!(
            "{prefix}_receive_errors_total {}\n",
            snapshot.receive_errors
        ));

        output.push_str(&format!(
            "# HELP {prefix}_queue_depth Current queue depth\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_queue_depth gauge\n"));
        output.push_str(&format!("{prefix}_queue_depth {}\n", snapshot.queue_depth));

        output.push_str(&format!(
            "# HELP {prefix}_latency_microseconds Latency in microseconds\n"
        ));
        output.push_str(&format!("# TYPE {prefix}_latency_microseconds summary\n"));
        output.push_str(&format!(
            "{prefix}_latency_microseconds{{quantile=\"0.5\"}} {}\n",
            snapshot.p50_latency_us
        ));
        output.push_str(&format!(
            "{prefix}_latency_microseconds{{quantile=\"0.95\"}} {}\n",
            snapshot.p95_latency_us
        ));
        output.push_str(&format!(
            "{prefix}_latency_microseconds{{quantile=\"0.99\"}} {}\n",
            snapshot.p99_latency_us
        ));

        output.push_str(&format!(
            "# HELP {prefix}_throughput_messages_per_second Message throughput\n"
        ));
        output.push_str(&format!(
            "# TYPE {prefix}_throughput_messages_per_second gauge\n"
        ));
        output.push_str(&format!(
            "{prefix}_throughput_messages_per_second{{direction=\"send\"}} {:.2}\n",
            snapshot.send_throughput
        ));
        output.push_str(&format!(
            "{prefix}_throughput_messages_per_second{{direction=\"recv\"}} {:.2}\n",
            snapshot.recv_throughput
        ));

        output
    }

    fn ensure_started(&self) {
        let mut start = self.start_time.write();
        if start.is_none() {
            *start = Some(Instant::now());
        }
    }
}

/// A snapshot of metrics at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Send errors
    pub send_errors: u64,
    /// Receive errors
    pub receive_errors: u64,
    /// Current queue depth
    pub queue_depth: u64,
    /// Peak queue depth
    pub peak_queue_depth: u64,
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// Minimum latency in microseconds
    pub min_latency_us: Option<u64>,
    /// Maximum latency in microseconds
    pub max_latency_us: u64,
    /// 50th percentile latency
    pub p50_latency_us: u64,
    /// 95th percentile latency
    pub p95_latency_us: u64,
    /// 99th percentile latency
    pub p99_latency_us: u64,
    /// Elapsed time in seconds
    pub elapsed_secs: f64,
    /// Send throughput (messages/second)
    pub send_throughput: f64,
    /// Receive throughput (messages/second)
    pub recv_throughput: f64,
    /// Send bandwidth (bytes/second)
    pub send_bandwidth: f64,
    /// Receive bandwidth (bytes/second)
    pub recv_bandwidth: f64,
}

/// A simple histogram for latency distribution.
#[derive(Debug, Default)]
struct LatencyHistogram {
    // Buckets: 0-10us, 10-100us, 100us-1ms, 1-10ms, 10-100ms, 100ms-1s, 1s+
    buckets: [u64; 7],
    // For percentile calculation, keep sorted samples (up to a limit)
    samples: Vec<u64>,
    max_samples: usize,
}

impl LatencyHistogram {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            buckets: [0; 7],
            samples: Vec::new(),
            max_samples: 10000,
        }
    }

    fn record(&mut self, latency_us: u64) {
        // Update bucket
        let bucket = match latency_us {
            0..=10 => 0,
            11..=100 => 1,
            101..=1000 => 2,
            1001..=10000 => 3,
            10001..=100000 => 4,
            100001..=1000000 => 5,
            _ => 6,
        };
        self.buckets[bucket] += 1;

        // Store sample for percentile calculation
        if self.samples.len() < self.max_samples {
            self.samples.push(latency_us);
        } else {
            // Reservoir sampling
            let idx = rand_usize() % (self.samples.len() + 1);
            if idx < self.samples.len() {
                self.samples[idx] = latency_us;
            }
        }
    }

    fn percentile(&self, p: u8) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_unstable();

        let idx = ((p as f64 / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[idx]
    }

    fn reset(&mut self) {
        self.buckets = [0; 7];
        self.samples.clear();
    }
}

/// Simple pseudo-random number for reservoir sampling.
fn rand_usize() -> usize {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish() as usize
}

/// Trait for channels that support metrics.
pub trait MeteredChannel {
    /// Get the metrics for this channel.
    fn metrics(&self) -> &ChannelMetrics;
}

/// A wrapper that adds metrics to any channel.
pub struct MeteredWrapper<C> {
    inner: C,
    metrics: ChannelMetrics,
}

impl<C> MeteredWrapper<C> {
    /// Create a new metered wrapper around a channel.
    pub fn new(channel: C) -> Self {
        Self {
            inner: channel,
            metrics: ChannelMetrics::new(),
        }
    }

    /// Get a reference to the inner channel.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Get a mutable reference to the inner channel.
    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// Consume the wrapper and return the inner channel.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C> MeteredChannel for MeteredWrapper<C> {
    fn metrics(&self) -> &ChannelMetrics {
        &self.metrics
    }
}

/// Extension trait for adding metrics to channels.
pub trait WithMetrics: Sized {
    /// Wrap this channel with metrics tracking.
    fn with_metrics(self) -> MeteredWrapper<Self> {
        MeteredWrapper::new(self)
    }
}

// Implement for all types
impl<T> WithMetrics for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_metrics() {
        let metrics = ChannelMetrics::new();

        metrics.record_send(100);
        metrics.record_send(200);
        metrics.record_recv(150);

        assert_eq!(metrics.messages_sent(), 2);
        assert_eq!(metrics.messages_received(), 1);
        assert_eq!(metrics.bytes_sent(), 300);
        assert_eq!(metrics.bytes_received(), 150);
    }

    #[test]
    fn test_error_tracking() {
        let metrics = ChannelMetrics::new();

        metrics.record_send_error();
        metrics.record_send_error();
        metrics.record_recv_error();

        assert_eq!(metrics.send_errors(), 2);
        assert_eq!(metrics.receive_errors(), 1);
    }

    #[test]
    fn test_latency_tracking() {
        let metrics = ChannelMetrics::new();

        metrics.record_latency(Duration::from_micros(100));
        metrics.record_latency(Duration::from_micros(200));
        metrics.record_latency(Duration::from_micros(300));

        assert_eq!(metrics.avg_latency_us(), 200);
        assert_eq!(metrics.min_latency_us(), Some(100));
        assert_eq!(metrics.max_latency_us(), 300);
    }

    #[test]
    fn test_queue_depth() {
        let metrics = ChannelMetrics::new();

        metrics.set_queue_depth(5);
        assert_eq!(metrics.queue_depth(), 5);
        assert_eq!(metrics.peak_queue_depth(), 5);

        metrics.set_queue_depth(10);
        assert_eq!(metrics.queue_depth(), 10);
        assert_eq!(metrics.peak_queue_depth(), 10);

        metrics.set_queue_depth(3);
        assert_eq!(metrics.queue_depth(), 3);
        assert_eq!(metrics.peak_queue_depth(), 10); // Peak unchanged
    }

    #[test]
    fn test_snapshot() {
        let metrics = ChannelMetrics::new();
        metrics.record_send(100);
        metrics.record_recv(50);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_sent, 1);
        assert_eq!(snapshot.messages_received, 1);
        assert_eq!(snapshot.bytes_sent, 100);
        assert_eq!(snapshot.bytes_received, 50);
    }

    #[test]
    fn test_json_export() {
        let metrics = ChannelMetrics::new();
        metrics.record_send(100);

        let json = metrics.to_json();
        assert!(json.contains("messages_sent"));
        assert!(json.contains("1"));
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = ChannelMetrics::new();
        metrics.record_send(100);

        let prom = metrics.to_prometheus("ipckit");
        assert!(prom.contains("ipckit_messages_sent_total 1"));
    }

    #[test]
    fn test_reset() {
        let metrics = ChannelMetrics::new();
        metrics.record_send(100);
        metrics.record_recv(50);

        metrics.reset();

        assert_eq!(metrics.messages_sent(), 0);
        assert_eq!(metrics.messages_received(), 0);
        assert_eq!(metrics.bytes_sent(), 0);
        assert_eq!(metrics.bytes_received(), 0);
    }

    #[test]
    fn test_with_metrics() {
        struct DummyChannel;

        let wrapped = DummyChannel.with_metrics();
        wrapped.metrics().record_send(100);
        assert_eq!(wrapped.metrics().messages_sent(), 1);
    }
}
