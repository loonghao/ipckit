"""Verify that implemented PyO3 bindings are available from ``ipckit``."""

import ipckit


EXPECTED_EXPORTS = {
    "AnonymousPipe",
    "NamedPipe",
    "SharedMemory",
    "IpcChannel",
    "FileChannel",
    "LocalSocketListener",
    "LocalSocketStream",
    "GracefulNamedPipe",
    "GracefulIpcChannel",
    "CliBridge",
    "CliBridgeConfig",
    "ProgressInfo",
    "CommandOutput",
    "wrap_command",
    "parse_progress",
    "ChannelMetrics",
    "MetricsSnapshot",
    "Event",
    "EventFilter",
    "EventBusConfig",
    "EventBus",
    "EventPublisher",
    "EventSubscriber",
    "TaskStatus",
    "TaskInfo",
    "CancellationToken",
    "TaskBuilder",
    "TaskFilter",
    "TaskHandle",
    "TaskManagerConfig",
    "TaskManager",
    "ApiServerConfig",
    "Request",
    "Response",
    "ApiClient",
    "json_dumps",
    "json_dumps_pretty",
    "json_loads",
    "__version__",
}


def test_python_bindings_are_exported_from_package():
    missing = sorted(name for name in EXPECTED_EXPORTS if not hasattr(ipckit, name))

    assert not missing, f"Missing package exports: {', '.join(missing)}"
    assert EXPECTED_EXPORTS.issubset(set(ipckit.__all__))
