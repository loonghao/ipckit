"""Pytest configuration and fixtures."""

import pytest


@pytest.fixture(autouse=True)
def cleanup_timeout():
    """Ensure tests don't hang indefinitely."""
    yield
    # Cleanup is handled by pytest-timeout
