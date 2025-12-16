"""Tests for CLI Bridge functionality."""

import os
import sys
import tempfile

import pytest


class TestProgressInfo:
    """Tests for ProgressInfo class."""

    def test_progress_info_creation(self):
        """Test creating ProgressInfo."""
        from ipckit import ProgressInfo

        info = ProgressInfo(50, 100)
        assert info.current == 50
        assert info.total == 100
        assert info.percentage == 50
        assert info.message is None

    def test_progress_info_with_message(self):
        """Test ProgressInfo with message."""
        from ipckit import ProgressInfo

        info = ProgressInfo(75, 100, "Almost done")
        assert info.current == 75
        assert info.total == 100
        assert info.percentage == 75
        assert info.message == "Almost done"

    def test_progress_info_zero_total(self):
        """Test ProgressInfo with zero total."""
        from ipckit import ProgressInfo

        info = ProgressInfo(50, 0)
        assert info.percentage == 0

    def test_progress_info_repr(self):
        """Test ProgressInfo string representation."""
        from ipckit import ProgressInfo

        info = ProgressInfo(50, 100)
        repr_str = repr(info)
        assert "ProgressInfo" in repr_str
        assert "50" in repr_str
        assert "100" in repr_str


class TestCliBridgeConfig:
    """Tests for CliBridgeConfig class."""

    def test_config_default(self):
        """Test default configuration."""
        from ipckit import CliBridgeConfig

        config = CliBridgeConfig()
        assert config.auto_register is True

    def test_config_custom(self):
        """Test custom configuration."""
        from ipckit import CliBridgeConfig

        config = CliBridgeConfig(
            server_url="/tmp/test.sock",
            auto_register=False,
            capture_stdout=True,
            capture_stderr=False,
        )
        assert config.server_url == "/tmp/test.sock"
        assert config.auto_register is False

    def test_config_from_env(self):
        """Test configuration from environment."""
        from ipckit import CliBridgeConfig

        # Set environment variable
        old_value = os.environ.get("IPCKIT_SERVER_URL")
        os.environ["IPCKIT_SERVER_URL"] = "/custom/path.sock"

        try:
            config = CliBridgeConfig.from_env()
            assert config.server_url == "/custom/path.sock"
        finally:
            if old_value is not None:
                os.environ["IPCKIT_SERVER_URL"] = old_value
            else:
                os.environ.pop("IPCKIT_SERVER_URL", None)

    def test_config_setters(self):
        """Test configuration setters."""
        from ipckit import CliBridgeConfig

        config = CliBridgeConfig()
        config.server_url = "/new/path.sock"
        config.auto_register = False

        assert config.server_url == "/new/path.sock"
        assert config.auto_register is False


class TestCliBridge:
    """Tests for CliBridge class."""

    def test_bridge_creation(self):
        """Test creating CliBridge."""
        from ipckit import CliBridge

        bridge = CliBridge()
        assert bridge.task_id is None
        assert bridge.is_cancelled is False

    def test_bridge_with_config(self):
        """Test creating CliBridge with config."""
        from ipckit import CliBridge, CliBridgeConfig

        config = CliBridgeConfig(auto_register=False)
        bridge = CliBridge(config)
        assert bridge.task_id is None

    def test_bridge_register_task(self):
        """Test registering a task."""
        from ipckit import CliBridge

        bridge = CliBridge()
        task_id = bridge.register_task("Test Task", "test")

        assert task_id.startswith("cli-")
        assert bridge.task_id == task_id

    def test_bridge_set_progress(self):
        """Test setting progress."""
        from ipckit import CliBridge

        bridge = CliBridge()
        bridge.register_task("Test", "test")

        # Should not raise
        bridge.set_progress(50)
        bridge.set_progress(75, "Almost done")

    def test_bridge_logging(self):
        """Test logging methods."""
        from ipckit import CliBridge

        bridge = CliBridge()
        bridge.register_task("Test", "test")

        # Should not raise
        bridge.log("info", "Test message")
        bridge.log("warn", "Warning message")
        bridge.log("error", "Error message")

    def test_bridge_stdout_stderr(self):
        """Test stdout/stderr methods."""
        from ipckit import CliBridge

        bridge = CliBridge()
        bridge.register_task("Test", "test")

        # Should not raise
        bridge.stdout("Output line")
        bridge.stderr("Error line")

    def test_bridge_complete(self):
        """Test completing a task."""
        from ipckit import CliBridge

        bridge = CliBridge()
        bridge.register_task("Test", "test")
        bridge.complete({"success": True, "count": 42})

    def test_bridge_fail(self):
        """Test failing a task."""
        from ipckit import CliBridge

        bridge = CliBridge()
        bridge.register_task("Test", "test")
        bridge.fail("Something went wrong")

    def test_bridge_context_manager(self):
        """Test using CliBridge as context manager."""
        from ipckit import CliBridge

        with CliBridge() as bridge:
            bridge.register_task("Test", "test")
            bridge.set_progress(100)

    def test_bridge_context_manager_exception(self):
        """Test context manager with exception."""
        from ipckit import CliBridge

        try:
            with CliBridge() as bridge:
                bridge.register_task("Test", "test")
                raise ValueError("Test error")
        except ValueError:
            pass  # Expected


class TestParseProgress:
    """Tests for parse_progress function."""

    def test_parse_percentage(self):
        """Test parsing percentage."""
        from ipckit import parse_progress

        info = parse_progress("50%", "percentage")
        assert info is not None
        assert info.percentage == 50

    def test_parse_percentage_with_text(self):
        """Test parsing percentage with surrounding text."""
        from ipckit import parse_progress

        info = parse_progress("Downloading... 75% complete", "percentage")
        assert info is not None
        assert info.percentage == 75

    def test_parse_fraction(self):
        """Test parsing fraction."""
        from ipckit import parse_progress

        info = parse_progress("5/10", "fraction")
        assert info is not None
        assert info.current == 5
        assert info.total == 10
        assert info.percentage == 50

    def test_parse_fraction_with_text(self):
        """Test parsing fraction with surrounding text."""
        from ipckit import parse_progress

        info = parse_progress("[3/4] Installing packages...", "fraction")
        assert info is not None
        assert info.current == 3
        assert info.total == 4
        assert info.percentage == 75

    def test_parse_progress_bar(self):
        """Test parsing progress bar."""
        from ipckit import parse_progress

        info = parse_progress("[=====>    ] 50%", "progress_bar")
        assert info is not None
        assert info.percentage == 50

    def test_parse_all(self):
        """Test parsing with all parsers."""
        from ipckit import parse_progress

        # Should match percentage
        info = parse_progress("Progress: 60%", "all")
        assert info is not None
        assert info.percentage == 60

        # Should match fraction
        info = parse_progress("Step 3/5", "all")
        assert info is not None
        assert info.percentage == 60

    def test_parse_no_match(self):
        """Test parsing with no match."""
        from ipckit import parse_progress

        info = parse_progress("Just some text", "all")
        assert info is None


class TestWrapCommand:
    """Tests for wrap_command function."""

    @pytest.mark.skipif(sys.platform != "win32", reason="Windows-specific test")
    def test_wrap_command_echo_windows(self):
        """Test wrapping echo command on Windows."""
        from ipckit import wrap_command

        output = wrap_command(
            ["cmd", "/C", "echo", "hello"],
            task_name="Echo Test",
            task_type="test",
        )

        assert output.exit_code == 0
        assert output.success is True
        assert "hello" in output.stdout

    @pytest.mark.skipif(sys.platform == "win32", reason="Unix-specific test")
    def test_wrap_command_echo_unix(self):
        """Test wrapping echo command on Unix."""
        from ipckit import wrap_command

        output = wrap_command(
            ["echo", "hello"],
            task_name="Echo Test",
            task_type="test",
        )

        assert output.exit_code == 0
        assert output.success is True
        assert "hello" in output.stdout

    @pytest.mark.skipif(sys.platform != "win32", reason="Windows-specific test")
    def test_wrap_command_failure_windows(self):
        """Test wrapping failing command on Windows."""
        from ipckit import wrap_command

        output = wrap_command(
            ["cmd", "/C", "exit", "1"],
            task_name="Fail Test",
            task_type="test",
        )

        assert output.exit_code == 1
        assert output.success is False

    @pytest.mark.skipif(sys.platform == "win32", reason="Unix-specific test")
    def test_wrap_command_failure_unix(self):
        """Test wrapping failing command on Unix."""
        from ipckit import wrap_command

        output = wrap_command(
            ["sh", "-c", "exit 1"],
            task_name="Fail Test",
            task_type="test",
        )

        assert output.exit_code == 1
        assert output.success is False

    def test_wrap_command_empty_args(self):
        """Test wrap_command with empty args raises error."""
        from ipckit import wrap_command

        with pytest.raises(ValueError):
            wrap_command([])

    def test_wrap_command_with_env(self):
        """Test wrap_command with environment variables."""
        from ipckit import wrap_command

        if sys.platform == "win32":
            output = wrap_command(
                ["cmd", "/C", "echo", "%MY_VAR%"],
                env={"MY_VAR": "test_value"},
            )
        else:
            output = wrap_command(
                ["sh", "-c", "echo $MY_VAR"],
                env={"MY_VAR": "test_value"},
            )

        assert output.exit_code == 0

    def test_wrap_command_with_cwd(self):
        """Test wrap_command with working directory."""
        from ipckit import wrap_command

        with tempfile.TemporaryDirectory() as tmpdir:
            if sys.platform == "win32":
                output = wrap_command(
                    ["cmd", "/C", "cd"],
                    cwd=tmpdir,
                )
            else:
                output = wrap_command(
                    ["pwd"],
                    cwd=tmpdir,
                )

            assert output.exit_code == 0

    def test_command_output_repr(self):
        """Test CommandOutput string representation."""
        from ipckit import wrap_command

        if sys.platform == "win32":
            output = wrap_command(["cmd", "/C", "echo", "test"])
        else:
            output = wrap_command(["echo", "test"])

        repr_str = repr(output)
        assert "CommandOutput" in repr_str
        assert "exit_code" in repr_str

    def test_command_output_duration(self):
        """Test CommandOutput duration tracking."""
        from ipckit import wrap_command

        if sys.platform == "win32":
            output = wrap_command(["cmd", "/C", "echo", "test"])
        else:
            output = wrap_command(["echo", "test"])

        assert output.duration_ms >= 0


class TestE2EScenarios:
    """End-to-end test scenarios."""

    def test_full_task_lifecycle(self):
        """Test complete task lifecycle."""
        from ipckit import CliBridge

        bridge = CliBridge()
        task_id = bridge.register_task("Build Project", "build")

        # Simulate build progress
        for i in range(0, 101, 10):
            bridge.set_progress(i, f"Step {i}%")

        bridge.complete({"built": True, "artifacts": ["main.exe"]})

        assert bridge.task_id == task_id

    def test_task_with_cancellation_check(self):
        """Test task with cancellation checking."""
        from ipckit import CliBridge

        bridge = CliBridge()
        bridge.register_task("Long Task", "process")

        steps_completed = 0
        for i in range(10):
            if bridge.is_cancelled:
                break
            bridge.set_progress(i * 10)
            steps_completed += 1

        bridge.complete({"steps": steps_completed})
        assert steps_completed == 10

    def test_multiple_bridges(self):
        """Test multiple bridges can coexist."""
        import time

        from ipckit import CliBridge

        bridge1 = CliBridge()
        task_id1 = bridge1.register_task("Task 1", "test")

        # Small delay to ensure different timestamp
        time.sleep(0.002)

        bridge2 = CliBridge()
        task_id2 = bridge2.register_task("Task 2", "test")

        assert task_id1 != task_id2
        assert bridge1.task_id == task_id1
        assert bridge2.task_id == task_id2

    def test_progress_parsing_integration(self):
        """Test progress parsing with real-world output patterns."""
        from ipckit import parse_progress

        # npm-style progress
        info = parse_progress("added 150 packages in 5s", "all")
        # No match expected for this pattern
        assert info is None

        # pip-style progress
        info = parse_progress("Downloading package... 45%", "all")
        assert info is not None
        assert info.percentage == 45

        # cargo-style progress
        info = parse_progress("   Compiling ipckit v0.1.0 (1/10)", "all")
        assert info is not None
        assert info.percentage == 10

        # git-style progress
        info = parse_progress("Receiving objects:  75% (150/200)", "all")
        assert info is not None
        assert info.percentage == 75


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
