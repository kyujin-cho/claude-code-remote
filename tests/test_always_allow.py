"""Tests for always allow manager module."""

import json
import tempfile
from pathlib import Path

from claude_code_telegram.always_allow import AlwaysAllowManager


class TestAlwaysAllowManager:
    """Tests for AlwaysAllowManager class."""

    def test_init_creates_file(self) -> None:
        """Test that initialization creates the storage file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            AlwaysAllowManager(storage_path)
            assert storage_path.exists()
            with open(storage_path) as f:
                data = json.load(f)
            assert data == {"tools": []}

    def test_add_tool(self) -> None:
        """Test adding a tool to always-allow list."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            manager.add_tool("Bash")
            assert manager.is_allowed("Bash")

    def test_add_tool_no_duplicates(self) -> None:
        """Test that adding the same tool twice doesn't create duplicates."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            manager.add_tool("Bash")
            manager.add_tool("Bash")
            assert manager.get_allowed_tools() == ["Bash"]

    def test_remove_tool(self) -> None:
        """Test removing a tool from always-allow list."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            manager.add_tool("Bash")
            manager.add_tool("Edit")
            manager.remove_tool("Bash")
            assert not manager.is_allowed("Bash")
            assert manager.is_allowed("Edit")

    def test_remove_nonexistent_tool(self) -> None:
        """Test removing a tool that doesn't exist doesn't raise error."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            manager.remove_tool("NonExistent")  # Should not raise

    def test_is_allowed_false_for_unknown_tool(self) -> None:
        """Test is_allowed returns False for unknown tools."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            assert not manager.is_allowed("Unknown")

    def test_get_allowed_tools(self) -> None:
        """Test getting the list of allowed tools."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            manager.add_tool("Bash")
            manager.add_tool("Edit")
            manager.add_tool("Write")
            tools = manager.get_allowed_tools()
            assert set(tools) == {"Bash", "Edit", "Write"}

    def test_clear(self) -> None:
        """Test clearing all allowed tools."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            manager = AlwaysAllowManager(storage_path)
            manager.add_tool("Bash")
            manager.add_tool("Edit")
            manager.clear()
            assert manager.get_allowed_tools() == []

    def test_persistence(self) -> None:
        """Test that preferences persist across manager instances."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"

            # First instance
            manager1 = AlwaysAllowManager(storage_path)
            manager1.add_tool("Bash")

            # Second instance reads the same file
            manager2 = AlwaysAllowManager(storage_path)
            assert manager2.is_allowed("Bash")

    def test_handles_corrupted_file(self) -> None:
        """Test that corrupted JSON file is handled gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            storage_path = Path(tmpdir) / "always_allow.json"
            storage_path.write_text("invalid json {{{")

            manager = AlwaysAllowManager(storage_path)
            assert manager.get_allowed_tools() == []
