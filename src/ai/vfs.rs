use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Virtual filesystem for the agent — stores markdown files, notes,
/// scripts, and working documents.
/// The agent can read/write these to maintain its own persistent memory.
pub struct VirtualFs {
    /// In-memory cache of files
    files: HashMap<String, VfsFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsFile {
    pub path: String,
    pub content: String,
    pub created_week: u32,
    pub modified_week: u32,
}

impl VirtualFs {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Write a file (create or overwrite)
    pub fn write(&mut self, path: &str, content: &str, week: u32) {
        let file = self
            .files
            .entry(path.to_string())
            .or_insert_with(|| VfsFile {
                path: path.to_string(),
                content: String::new(),
                created_week: week,
                modified_week: week,
            });
        file.content = content.to_string();
        file.modified_week = week;
    }

    /// Read a file
    pub fn read(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(|f| f.content.as_str())
    }

    /// Append to a file
    pub fn append(&mut self, path: &str, content: &str, week: u32) {
        let file = self
            .files
            .entry(path.to_string())
            .or_insert_with(|| VfsFile {
                path: path.to_string(),
                content: String::new(),
                created_week: week,
                modified_week: week,
            });
        file.content.push_str(content);
        file.modified_week = week;
    }

    /// Delete a file
    pub fn delete(&mut self, path: &str) -> bool {
        self.files.remove(path).is_some()
    }

    /// List all files (optionally filtered by prefix/directory)
    pub fn list(&self, prefix: Option<&str>) -> Vec<&str> {
        self.files
            .keys()
            .filter(|k| match prefix {
                Some(p) => k.starts_with(p),
                None => true,
            })
            .map(|k| k.as_str())
            .collect()
    }

    /// Check if file exists
    pub fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Save all files to a YAML file in the given directory.
    pub fn save_to_dir(&self, dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(dir)?;
        let yaml = serde_yaml::to_string(&self.files)?;
        std::fs::write(dir.join("vfs.yaml"), yaml)?;
        Ok(())
    }

    /// Load from a YAML file in the given directory.
    pub fn load_from_dir(dir: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let vfs_path = dir.join("vfs.yaml");
        let files = if vfs_path.exists() {
            let content = std::fs::read_to_string(&vfs_path)?;
            serde_yaml::from_str(&content)?
        } else {
            HashMap::new()
        };
        Ok(Self { files })
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read() {
        let mut vfs = VirtualFs::new();
        vfs.write("notes/davis.md", "# Davis\nRival. Blocking zoning.", 1);
        assert_eq!(
            vfs.read("notes/davis.md"),
            Some("# Davis\nRival. Blocking zoning.")
        );
    }

    #[test]
    fn test_append() {
        let mut vfs = VirtualFs::new();
        vfs.write("log.md", "Week 1: Started.\n", 1);
        vfs.append("log.md", "Week 2: Met Davis.\n", 2);
        let content = vfs.read("log.md").unwrap();
        assert!(content.contains("Week 1"));
        assert!(content.contains("Week 2"));
    }

    #[test]
    fn test_list() {
        let mut vfs = VirtualFs::new();
        vfs.write("notes/davis.md", "rival", 1);
        vfs.write("notes/kowalski.md", "ally", 1);
        vfs.write("drafts/bill1.md", "law text", 1);

        let notes = vfs.list(Some("notes/"));
        assert_eq!(notes.len(), 2);

        let all = vfs.list(None);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_delete() {
        let mut vfs = VirtualFs::new();
        vfs.write("temp.md", "temporary", 1);
        assert!(vfs.delete("temp.md"));
        assert!(!vfs.exists("temp.md"));
    }

    #[test]
    fn test_overwrite() {
        let mut vfs = VirtualFs::new();
        vfs.write("file.md", "version 1", 1);
        vfs.write("file.md", "version 2", 2);
        assert_eq!(vfs.read("file.md"), Some("version 2"));
    }

    #[test]
    fn test_yaml_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut vfs = VirtualFs::new();
        vfs.write("notes/test.md", "hello world", 1);
        vfs.write("log.md", "week 1", 1);
        vfs.save_to_dir(dir.path()).unwrap();

        let loaded = VirtualFs::load_from_dir(dir.path()).unwrap();
        assert_eq!(loaded.file_count(), 2);
        assert_eq!(loaded.read("notes/test.md"), Some("hello world"));
        assert_eq!(loaded.read("log.md"), Some("week 1"));
    }
}
