"""Tests for hook handler module."""

from claude_code_telegram.hook_handler import PermissionRequest, create_hook_response


class TestPermissionRequest:
    """Tests for PermissionRequest class."""

    def test_from_hook_input_bash(self) -> None:
        """Test parsing Bash tool input."""
        data = {
            "tool_name": "Bash",
            "tool_input": {"command": "ls -la"},
        }
        request = PermissionRequest.from_hook_input(data)
        assert request.tool_name == "Bash"
        assert request.tool_input["command"] == "ls -la"
        assert len(request.request_id) == 8

    def test_from_hook_input_edit(self) -> None:
        """Test parsing Edit tool input."""
        data = {
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "/test/file.py",
                "old_string": "old",
                "new_string": "new",
            },
        }
        request = PermissionRequest.from_hook_input(data)
        assert request.tool_name == "Edit"
        assert request.tool_input["file_path"] == "/test/file.py"

    def test_from_hook_input_unknown(self) -> None:
        """Test parsing unknown tool input."""
        data = {}
        request = PermissionRequest.from_hook_input(data)
        assert request.tool_name == "unknown"
        assert request.tool_input == {}

    def test_format_message_bash(self) -> None:
        """Test formatting Bash command message."""
        request = PermissionRequest(
            tool_name="Bash",
            tool_input={"command": "echo hello"},
            request_id="abc12345",
        )
        message = request.format_message()
        assert "Permission Request" in message
        assert "Bash" in message
        assert "echo hello" in message
        assert "abc12345" in message

    def test_format_message_with_hostname(self) -> None:
        """Test formatting message includes hostname when provided."""
        request = PermissionRequest(
            tool_name="Bash",
            tool_input={"command": "ls"},
            request_id="abc12345",
        )
        message = request.format_message(hostname="my-macbook")
        assert "my-macbook" in message
        assert "Host" in message

    def test_format_message_without_hostname(self) -> None:
        """Test formatting message without hostname."""
        request = PermissionRequest(
            tool_name="Bash",
            tool_input={"command": "ls"},
            request_id="abc12345",
        )
        message = request.format_message()
        assert "Host" not in message

    def test_format_message_edit(self) -> None:
        """Test formatting Edit tool message."""
        request = PermissionRequest(
            tool_name="Edit",
            tool_input={
                "file_path": "/test.py",
                "old_string": "foo",
                "new_string": "bar",
            },
            request_id="xyz98765",
        )
        message = request.format_message()
        assert "Edit" in message
        assert "/test.py" in message
        assert "foo" in message
        assert "bar" in message


class TestCreateHookResponse:
    """Tests for create_hook_response function."""

    def test_allow_response(self) -> None:
        """Test allow decision response."""
        response = create_hook_response("allow")
        assert response["hookSpecificOutput"]["decision"]["behavior"] == "allow"

    def test_deny_response(self) -> None:
        """Test deny decision response."""
        response = create_hook_response("deny")
        assert response["hookSpecificOutput"]["decision"]["behavior"] == "deny"

    def test_invalid_response_defaults_to_deny(self) -> None:
        """Test that invalid decisions default to deny."""
        response = create_hook_response("invalid")
        assert response["hookSpecificOutput"]["decision"]["behavior"] == "deny"
