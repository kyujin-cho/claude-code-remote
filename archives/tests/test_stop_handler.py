"""Tests for stop handler module."""

import json
import tempfile
from pathlib import Path

from claude_code_telegram.stop_handler import StopEvent


class TestStopEvent:
    """Tests for StopEvent class."""

    def test_from_hook_input(self) -> None:
        """Test parsing stop event from hook input."""
        data = {
            "session_id": "abc123",
            "transcript_path": "/path/to/transcript.jsonl",
            "cwd": "/home/user/project",
            "stop_hook_active": False,
        }
        event = StopEvent.from_hook_input(data)
        assert event.session_id == "abc123"
        assert event.transcript_path == "/path/to/transcript.jsonl"
        assert event.cwd == "/home/user/project"
        assert event.stop_hook_active is False

    def test_from_hook_input_empty(self) -> None:
        """Test parsing empty hook input."""
        event = StopEvent.from_hook_input({})
        assert event.session_id == ""
        assert event.transcript_path == ""
        assert event.cwd == ""
        assert event.stop_hook_active is False

    def test_from_hook_input_stop_hook_active(self) -> None:
        """Test parsing when stop_hook_active is True."""
        data = {"stop_hook_active": True}
        event = StopEvent.from_hook_input(data)
        assert event.stop_hook_active is True

    def test_get_last_assistant_message_no_path(self) -> None:
        """Test get_last_assistant_message with no transcript path."""
        event = StopEvent(
            session_id="abc",
            transcript_path="",
            cwd="/home",
            stop_hook_active=False,
        )
        assert event.get_last_assistant_message() is None

    def test_get_last_assistant_message_nonexistent_file(self) -> None:
        """Test get_last_assistant_message with nonexistent file."""
        event = StopEvent(
            session_id="abc",
            transcript_path="/nonexistent/path.jsonl",
            cwd="/home",
            stop_hook_active=False,
        )
        assert event.get_last_assistant_message() is None

    def test_get_last_assistant_message_valid_transcript(self) -> None:
        """Test get_last_assistant_message with valid transcript."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
            # Write some transcript entries
            f.write(json.dumps({
                "type": "user",
                "message": {"content": [{"type": "text", "text": "Hello"}]}
            }) + "\n")
            f.write(json.dumps({
                "type": "assistant",
                "message": {"content": [{"type": "text", "text": "First response"}]}
            }) + "\n")
            f.write(json.dumps({
                "type": "assistant",
                "message": {"content": [{"type": "text", "text": "Final response"}]}
            }) + "\n")
            transcript_path = f.name

        try:
            event = StopEvent(
                session_id="abc",
                transcript_path=transcript_path,
                cwd="/home",
                stop_hook_active=False,
            )
            last_message = event.get_last_assistant_message()
            assert last_message == "Final response"
        finally:
            Path(transcript_path).unlink()

    def test_get_last_assistant_message_no_assistant_entries(self) -> None:
        """Test get_last_assistant_message with no assistant messages."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False) as f:
            f.write(json.dumps({
                "type": "user",
                "message": {"content": [{"type": "text", "text": "Hello"}]}
            }) + "\n")
            transcript_path = f.name

        try:
            event = StopEvent(
                session_id="abc",
                transcript_path=transcript_path,
                cwd="/home",
                stop_hook_active=False,
            )
            assert event.get_last_assistant_message() is None
        finally:
            Path(transcript_path).unlink()
