//! Always-allow manager for persistent tool preferences.
//!
//! Manages a whitelist of tools that should be automatically approved.

use crate::config::default_always_allow_path;
use crate::error::AlwaysAllowError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Storage format for always-allow preferences.
#[derive(Debug, Serialize, Deserialize, Default)]
struct AlwaysAllowData {
    #[serde(default)]
    tools: Vec<String>,
}

/// Manager for always-allow tool preferences.
#[derive(Debug, Clone)]
pub struct AlwaysAllowManager {
    storage_path: PathBuf,
}

impl AlwaysAllowManager {
    /// Create a new manager with the given storage path.
    pub fn new(storage_path: Option<PathBuf>) -> Self {
        let path = storage_path.unwrap_or_else(default_always_allow_path);
        Self { storage_path: path }
    }

    /// Ensure the storage file exists.
    fn ensure_storage_exists(&self) -> Result<(), AlwaysAllowError> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if !self.storage_path.exists() {
            let data = AlwaysAllowData::default();
            let content = serde_json::to_string_pretty(&data)?;
            fs::write(&self.storage_path, content)?;
        }

        Ok(())
    }

    /// Read data from storage file.
    fn read_data(&self) -> AlwaysAllowData {
        match fs::read_to_string(&self.storage_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AlwaysAllowData::default(),
        }
    }

    /// Write data to storage file.
    fn write_data(&self, data: &AlwaysAllowData) -> Result<(), AlwaysAllowError> {
        self.ensure_storage_exists()?;
        let content = serde_json::to_string_pretty(data)?;
        fs::write(&self.storage_path, content)?;
        Ok(())
    }

    /// Check if a tool is in the always-allow list.
    pub fn is_allowed(&self, tool_name: &str) -> bool {
        let data = self.read_data();
        data.tools.contains(&tool_name.to_string())
    }

    /// Add a tool to the always-allow list.
    pub fn add_tool(&self, tool_name: &str) -> Result<(), AlwaysAllowError> {
        let mut data = self.read_data();
        let tool = tool_name.to_string();

        if !data.tools.contains(&tool) {
            data.tools.push(tool);
            self.write_data(&data)?;
        }

        Ok(())
    }

    /// Remove a tool from the always-allow list.
    pub fn remove_tool(&self, tool_name: &str) -> Result<(), AlwaysAllowError> {
        let mut data = self.read_data();
        data.tools.retain(|t| t != tool_name);
        self.write_data(&data)?;
        Ok(())
    }

    /// Get the list of always-allowed tools.
    pub fn get_allowed_tools(&self) -> Vec<String> {
        self.read_data().tools
    }

    /// Clear all always-allow preferences.
    pub fn clear(&self) -> Result<(), AlwaysAllowError> {
        let data = AlwaysAllowData::default();
        self.write_data(&data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_and_check_tool() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("always_allow.json");
        let manager = AlwaysAllowManager::new(Some(storage_path));

        assert!(!manager.is_allowed("Bash"));

        manager.add_tool("Bash").unwrap();
        assert!(manager.is_allowed("Bash"));
    }

    #[test]
    fn test_add_tool_no_duplicates() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("always_allow.json");
        let manager = AlwaysAllowManager::new(Some(storage_path));

        manager.add_tool("Bash").unwrap();
        manager.add_tool("Bash").unwrap();

        let tools = manager.get_allowed_tools();
        assert_eq!(tools.len(), 1);
    }

    #[test]
    fn test_remove_tool() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("always_allow.json");
        let manager = AlwaysAllowManager::new(Some(storage_path));

        manager.add_tool("Bash").unwrap();
        manager.add_tool("Edit").unwrap();
        assert!(manager.is_allowed("Bash"));

        manager.remove_tool("Bash").unwrap();
        assert!(!manager.is_allowed("Bash"));
        assert!(manager.is_allowed("Edit"));
    }

    #[test]
    fn test_clear() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("always_allow.json");
        let manager = AlwaysAllowManager::new(Some(storage_path));

        manager.add_tool("Bash").unwrap();
        manager.add_tool("Edit").unwrap();
        assert_eq!(manager.get_allowed_tools().len(), 2);

        manager.clear().unwrap();
        assert!(manager.get_allowed_tools().is_empty());
    }

    #[test]
    fn test_handles_missing_file() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("nonexistent").join("always_allow.json");
        let manager = AlwaysAllowManager::new(Some(storage_path));

        // Should not panic, returns empty list
        assert!(manager.get_allowed_tools().is_empty());
        assert!(!manager.is_allowed("Bash"));
    }

    #[test]
    fn test_persistence() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("always_allow.json");

        // Add tool with first manager
        {
            let manager = AlwaysAllowManager::new(Some(storage_path.clone()));
            manager.add_tool("Bash").unwrap();
        }

        // Check with new manager instance
        {
            let manager = AlwaysAllowManager::new(Some(storage_path));
            assert!(manager.is_allowed("Bash"));
        }
    }
}
