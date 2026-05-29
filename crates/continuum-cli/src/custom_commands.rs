//! Custom slash commands loaded from `.continuum/commands/` directory.
//!
//! Users can create custom commands by adding markdown files:
//!   .continuum/commands/review.md
//!   .continuum/commands/test.md
//!
//! Each file becomes a `/review` or `/test` command.
//! The file content is used as the prompt template.

use std::path::{Path, PathBuf};

/// A custom command loaded from a markdown file.
#[derive(Debug, Clone)]
pub struct CustomCommand {
    /// Command name (filename without extension).
    pub name: String,
    /// Path to the command file.
    pub path: PathBuf,
    /// Content of the command file (prompt template).
    pub content: String,
}

/// Load all custom commands from the `.continuum/commands/` directory.
#[allow(dead_code)]
pub fn load_commands(project_dir: &Path) -> Vec<CustomCommand> {
    let commands_dir = project_dir.join(".continuum").join("commands");

    if !commands_dir.exists() {
        return Vec::new();
    }

    let mut commands = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&commands_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        commands.push(CustomCommand {
                            name: name.to_string(),
                            path,
                            content,
                        });
                    }
                }
            }
        }
    }

    commands.sort_by(|a, b| a.name.cmp(&b.name));
    commands
}

/// Find a custom command by name.
pub fn find_command(project_dir: &Path, name: &str) -> Option<CustomCommand> {
    let commands_dir = project_dir.join(".continuum").join("commands");
    let path = commands_dir.join(format!("{name}.md"));

    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Some(CustomCommand {
                name: name.to_string(),
                path,
                content,
            });
        }
    }

    None
}

/// Create a sample custom command for demonstration.
pub fn create_sample_command(project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let commands_dir = project_dir.join(".continuum").join("commands");
    std::fs::create_dir_all(&commands_dir)?;

    let sample = r#"# Code Review

Review the current changes for:
1. Security vulnerabilities
2. Performance issues
3. Error handling
4. Code style consistency

Focus on production readiness.
"#;

    let sample_path = commands_dir.join("review.md");
    if !sample_path.exists() {
        std::fs::write(&sample_path, sample)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_commands_empty_dir() {
        let dir = std::env::temp_dir().join("continuum_test_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let commands = load_commands(&dir);
        assert!(commands.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_commands_with_files() {
        let dir = std::env::temp_dir().join("continuum_test_cmds");
        let _ = fs::remove_dir_all(&dir);
        let commands_dir = dir.join(".continuum").join("commands");
        fs::create_dir_all(&commands_dir).unwrap();

        fs::write(commands_dir.join("review.md"), "# Review\nReview code").unwrap();
        fs::write(commands_dir.join("test.md"), "# Test\nWrite tests").unwrap();
        fs::write(commands_dir.join("notes.txt"), "not a command").unwrap();

        let commands = load_commands(&dir);
        assert_eq!(commands.len(), 2);
        assert!(commands.iter().any(|c| c.name == "review"));
        assert!(commands.iter().any(|c| c.name == "test"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_command() {
        let dir = std::env::temp_dir().join("continuum_test_find");
        let _ = fs::remove_dir_all(&dir);
        let commands_dir = dir.join(".continuum").join("commands");
        fs::create_dir_all(&commands_dir).unwrap();

        fs::write(commands_dir.join("review.md"), "# Review\nReview code").unwrap();

        let found = find_command(&dir, "review");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "review");

        let not_found = find_command(&dir, "nonexistent");
        assert!(not_found.is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
