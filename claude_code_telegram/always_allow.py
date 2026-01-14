"""Always Allow Manager for persistent tool preferences."""

import json
from pathlib import Path


class AlwaysAllowManager:
    """Manages always-allow preferences for tools."""

    DEFAULT_PATH = Path.home() / ".claude" / "always_allow.json"

    def __init__(self, storage_path: Path | None = None) -> None:
        self.storage_path = storage_path or self.DEFAULT_PATH
        self._ensure_storage_exists()

    def _ensure_storage_exists(self) -> None:
        """Ensure the storage directory and file exist."""
        self.storage_path.parent.mkdir(parents=True, exist_ok=True)
        if not self.storage_path.exists():
            self._write_data({"tools": []})

    def _read_data(self) -> dict[str, list[str]]:
        """Read data from storage file."""
        try:
            with open(self.storage_path) as f:
                data = json.load(f)
                if "tools" not in data:
                    data["tools"] = []
                return data
        except (json.JSONDecodeError, FileNotFoundError):
            return {"tools": []}

    def _write_data(self, data: dict[str, list[str]]) -> None:
        """Write data to storage file."""
        with open(self.storage_path, "w") as f:
            json.dump(data, f, indent=2)

    def is_allowed(self, tool_name: str) -> bool:
        """Check if a tool is in the always-allow list."""
        data = self._read_data()
        return tool_name in data["tools"]

    def add_tool(self, tool_name: str) -> None:
        """Add a tool to the always-allow list."""
        data = self._read_data()
        if tool_name not in data["tools"]:
            data["tools"].append(tool_name)
            self._write_data(data)

    def remove_tool(self, tool_name: str) -> None:
        """Remove a tool from the always-allow list."""
        data = self._read_data()
        if tool_name in data["tools"]:
            data["tools"].remove(tool_name)
            self._write_data(data)

    def get_allowed_tools(self) -> list[str]:
        """Get the list of always-allowed tools."""
        data = self._read_data()
        return data["tools"]

    def clear(self) -> None:
        """Clear all always-allow preferences."""
        self._write_data({"tools": []})
