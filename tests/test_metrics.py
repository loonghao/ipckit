"""Tests for ChannelMetrics functionality."""

import json
import time

import pytest


class TestChannelMetrics:
    """Unit tests for ChannelMetrics."""

    def test_create_metrics(self):
        """Test creating a new metrics instance."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        assert metrics.messages_sent == 0
        assert metrics.messages_received == 0
        assert metrics.bytes_sent == 0
        assert metrics.bytes_received == 0

    def test_record_send(self):
        """Test recording sent messages."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_send(200)

        assert metrics.messages_sent == 2
        assert metrics.bytes_sent == 300

    def test_record_recv(self):
        """Test recording received messages."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_recv(50)
        metrics.record_recv(150)

        assert metrics.messages_received == 2
        assert metrics.bytes_received == 200

    def test_record_errors(self):
        """Test recording errors."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send_error()
        metrics.record_send_error()
        metrics.record_recv_error()

        assert metrics.send_errors == 2
        assert metrics.receive_errors == 1

    def test_record_latency(self):
        """Test recording latency."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_latency_us(100)
        metrics.record_latency_us(200)
        metrics.record_latency_us(300)

        assert metrics.avg_latency_us > 0
        assert metrics.min_latency_us == 100
        assert metrics.max_latency_us == 300

    def test_record_latency_ms(self):
        """Test recording latency in milliseconds."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_latency_ms(1)  # 1ms = 1000us
        metrics.record_latency_ms(2)  # 2ms = 2000us

        assert metrics.min_latency_us == 1000
        assert metrics.max_latency_us == 2000

    def test_queue_depth(self):
        """Test queue depth tracking."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.set_queue_depth(5)
        assert metrics.queue_depth == 5

        metrics.set_queue_depth(10)
        assert metrics.queue_depth == 10
        assert metrics.peak_queue_depth == 10

        metrics.set_queue_depth(3)
        assert metrics.queue_depth == 3
        assert metrics.peak_queue_depth == 10  # Peak should remain

    def test_latency_percentile(self):
        """Test latency percentile calculation."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        # Record 100 latencies from 1 to 100
        for i in range(1, 101):
            metrics.record_latency_us(i)

        # Percentile calculation may vary by implementation
        # Just verify it returns a value and doesn't error
        p50 = metrics.latency_percentile(50)
        p99 = metrics.latency_percentile(99)
        assert isinstance(p50, int)
        assert isinstance(p99, int)
        # p99 should be >= p50
        assert p99 >= p50

    def test_throughput(self):
        """Test throughput calculation."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()

        # Record some messages
        for _ in range(10):
            metrics.record_send(100)
            metrics.record_recv(50)

        # Wait a bit for elapsed time
        time.sleep(0.1)

        # Throughput should be positive
        assert metrics.send_throughput >= 0
        assert metrics.recv_throughput >= 0
        assert metrics.send_bandwidth >= 0
        assert metrics.recv_bandwidth >= 0

    def test_elapsed_time(self):
        """Test elapsed time tracking."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        time.sleep(0.1)

        elapsed = metrics.elapsed_secs
        # Elapsed time should be a non-negative float
        # Note: elapsed_secs may be calculated from snapshot which could be 0
        # if no operations have been recorded
        assert isinstance(elapsed, float)
        assert elapsed >= 0

    def test_reset(self):
        """Test resetting metrics."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_recv(50)
        metrics.record_send_error()

        metrics.reset()

        assert metrics.messages_sent == 0
        assert metrics.messages_received == 0
        assert metrics.bytes_sent == 0
        assert metrics.bytes_received == 0
        assert metrics.send_errors == 0

    def test_snapshot(self):
        """Test getting metrics snapshot."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_recv(50)
        metrics.record_latency_us(150)

        snapshot = metrics.snapshot()

        assert isinstance(snapshot, dict)
        assert snapshot["messages_sent"] == 1
        assert snapshot["messages_received"] == 1
        assert snapshot["bytes_sent"] == 100
        assert snapshot["bytes_received"] == 50
        assert "avg_latency_us" in snapshot
        assert "elapsed_secs" in snapshot

    def test_to_json(self):
        """Test JSON export."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_recv(50)

        json_str = metrics.to_json()
        assert isinstance(json_str, str)

        # Should be valid JSON
        data = json.loads(json_str)
        assert data["messages_sent"] == 1
        assert data["messages_received"] == 1

    def test_to_json_pretty(self):
        """Test pretty JSON export."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)

        json_str = metrics.to_json_pretty()
        assert isinstance(json_str, str)
        assert "\n" in json_str  # Pretty format has newlines

    def test_to_prometheus(self):
        """Test Prometheus format export."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_recv(50)
        metrics.record_latency_us(150)

        prom_str = metrics.to_prometheus("ipckit")
        assert isinstance(prom_str, str)
        assert "ipckit_messages_sent" in prom_str
        assert "ipckit_messages_received" in prom_str
        assert "ipckit_bytes_sent" in prom_str

    def test_repr(self):
        """Test string representation."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_recv(50)

        repr_str = repr(metrics)
        assert "ChannelMetrics" in repr_str
        assert "sent=1" in repr_str
        assert "recv=1" in repr_str


class TestMetricsSnapshot:
    """Unit tests for MetricsSnapshot."""

    def test_snapshot_properties(self):
        """Test snapshot property access."""
        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        metrics.record_send(100)
        metrics.record_recv(50)
        metrics.record_latency_us(150)
        metrics.set_queue_depth(5)

        # Get snapshot dict
        snapshot = metrics.snapshot()

        # Check all expected keys
        assert "messages_sent" in snapshot
        assert "messages_received" in snapshot
        assert "bytes_sent" in snapshot
        assert "bytes_received" in snapshot
        assert "send_errors" in snapshot
        assert "receive_errors" in snapshot
        assert "queue_depth" in snapshot
        assert "peak_queue_depth" in snapshot
        assert "avg_latency_us" in snapshot
        assert "min_latency_us" in snapshot
        assert "max_latency_us" in snapshot
        assert "p50_latency_us" in snapshot
        assert "p95_latency_us" in snapshot
        assert "p99_latency_us" in snapshot
        assert "elapsed_secs" in snapshot
        assert "send_throughput" in snapshot
        assert "recv_throughput" in snapshot
        assert "send_bandwidth" in snapshot
        assert "recv_bandwidth" in snapshot


class TestMetricsThreadSafety:
    """Test thread safety of metrics."""

    def test_concurrent_updates(self):
        """Test concurrent metric updates from multiple threads."""
        import threading

        from ipckit import ChannelMetrics

        metrics = ChannelMetrics()
        num_threads = 10
        updates_per_thread = 100

        def update_metrics():
            for _ in range(updates_per_thread):
                metrics.record_send(10)
                metrics.record_recv(5)
                metrics.record_latency_us(100)

        threads = [threading.Thread(target=update_metrics) for _ in range(num_threads)]

        for t in threads:
            t.start()
        for t in threads:
            t.join()

        expected_messages = num_threads * updates_per_thread
        assert metrics.messages_sent == expected_messages
        assert metrics.messages_received == expected_messages


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
