"""Tests for API Server bindings."""

import pytest


class TestApiServerConfig:
    """Unit tests for ApiServerConfig."""

    def test_default_config(self):
        """Test creating default config."""
        from ipckit import ApiServerConfig

        config = ApiServerConfig()
        assert config.enable_cors is True
        assert isinstance(config.cors_origins, list)

    def test_custom_socket_path(self):
        """Test setting custom socket path."""
        from ipckit import ApiServerConfig

        config = ApiServerConfig(socket_path="/tmp/my_socket")
        assert config.socket_path == "/tmp/my_socket"

    def test_disable_cors(self):
        """Test disabling CORS."""
        from ipckit import ApiServerConfig

        config = ApiServerConfig(enable_cors=False)
        assert config.enable_cors is False

    def test_custom_cors_origins(self):
        """Test custom CORS origins."""
        from ipckit import ApiServerConfig

        origins = ["http://localhost:3000", "http://example.com"]
        config = ApiServerConfig(cors_origins=origins)
        assert config.cors_origins == origins

    def test_setters(self):
        """Test property setters."""
        from ipckit import ApiServerConfig

        config = ApiServerConfig()

        config.socket_path = "/new/path"
        assert config.socket_path == "/new/path"

        config.enable_cors = False
        assert config.enable_cors is False

        config.cors_origins = ["http://test.com"]
        assert config.cors_origins == ["http://test.com"]

    def test_repr(self):
        """Test string representation."""
        from ipckit import ApiServerConfig

        config = ApiServerConfig(socket_path="/test")
        repr_str = repr(config)
        assert "ApiServerConfig" in repr_str
        assert "/test" in repr_str


class TestResponse:
    """Unit tests for Response."""

    def test_default_response(self):
        """Test creating default response."""
        from ipckit import Response

        resp = Response()
        assert resp.status == 200

    def test_custom_status(self):
        """Test custom status code."""
        from ipckit import Response

        resp = Response(status=201)
        assert resp.status == 201

    def test_ok_response(self):
        """Test creating OK response."""
        from ipckit import Response

        resp = Response.ok({"data": [1, 2, 3]})
        assert resp.status == 200

    def test_created_response(self):
        """Test creating Created response."""
        from ipckit import Response

        resp = Response.created({"id": "new-item"})
        assert resp.status == 201

    def test_no_content_response(self):
        """Test creating No Content response."""
        from ipckit import Response

        resp = Response.no_content()
        assert resp.status == 204

    def test_bad_request_response(self):
        """Test creating Bad Request response."""
        from ipckit import Response

        resp = Response.bad_request("Invalid input")
        assert resp.status == 400

    def test_not_found_response(self):
        """Test creating Not Found response."""
        from ipckit import Response

        resp = Response.not_found()
        assert resp.status == 404

    def test_internal_error_response(self):
        """Test creating Internal Error response."""
        from ipckit import Response

        resp = Response.internal_error("Something went wrong")
        assert resp.status == 500

    def test_set_header(self):
        """Test setting headers."""
        from ipckit import Response

        resp = Response()
        resp.set_header("X-Custom-Header", "custom-value")
        # Header is set internally, just verify no error

    def test_set_json(self):
        """Test setting JSON body."""
        from ipckit import Response

        resp = Response()
        resp.set_json({"key": "value", "list": [1, 2, 3]})
        # Body is set internally, just verify no error

    def test_set_json_complex(self):
        """Test setting complex JSON body."""
        from ipckit import Response

        resp = Response()
        complex_data = {
            "string": "hello",
            "number": 42,
            "float": 3.14,
            "bool": True,
            "null": None,
            "list": [1, "two", 3.0],
            "nested": {"a": {"b": {"c": 1}}},
        }
        resp.set_json(complex_data)

    def test_repr(self):
        """Test string representation."""
        from ipckit import Response

        resp = Response(status=201)
        repr_str = repr(resp)
        assert "Response" in repr_str
        assert "201" in repr_str


class TestApiClient:
    """Unit tests for ApiClient."""

    def test_create_client(self):
        """Test creating API client."""
        from ipckit import ApiClient

        client = ApiClient("/tmp/test_socket")
        assert client is not None

    def test_connect_default(self):
        """Test connecting to default socket."""
        from ipckit import ApiClient

        client = ApiClient.connect()
        assert client is not None

    def test_repr(self):
        """Test string representation."""
        from ipckit import ApiClient

        client = ApiClient("/tmp/test")
        repr_str = repr(client)
        assert "ApiClient" in repr_str


class TestApiClientIntegration:
    """Integration tests for ApiClient (requires running server)."""

    @pytest.mark.skip(reason="Requires running API server")
    def test_get_request(self):
        """Test GET request."""
        from ipckit import ApiClient

        client = ApiClient.connect()
        result = client.get("/v1/health")
        assert result is not None

    @pytest.mark.skip(reason="Requires running API server")
    def test_post_request(self):
        """Test POST request."""
        from ipckit import ApiClient

        client = ApiClient.connect()
        result = client.post("/v1/tasks", {"name": "test-task"})
        assert result is not None

    @pytest.mark.skip(reason="Requires running API server")
    def test_put_request(self):
        """Test PUT request."""
        from ipckit import ApiClient

        client = ApiClient.connect()
        result = client.put("/v1/tasks/123", {"name": "updated-task"})
        assert result is not None

    @pytest.mark.skip(reason="Requires running API server")
    def test_delete_request(self):
        """Test DELETE request."""
        from ipckit import ApiClient

        client = ApiClient.connect()
        result = client.delete("/v1/tasks/123")
        assert result is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
