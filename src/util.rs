//! Shared utility functions.

use std::io::{self, Read};

/// Read all input from stdin as a string.
pub fn read_stdin() -> Result<String, io::Error> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Extract project name from a directory path.
pub fn project_name_from_cwd(cwd: &str) -> String {
    cwd.split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_name_from_cwd() {
        assert_eq!(project_name_from_cwd("/home/user/my-project"), "my-project");
        assert_eq!(project_name_from_cwd("/tmp"), "tmp");
        assert_eq!(project_name_from_cwd(""), "Unknown");
    }
}
