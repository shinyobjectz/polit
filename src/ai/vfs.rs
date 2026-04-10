use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::persistence::{Database, CF_WORLD_STATE};

/// Virtual filesystem for the agent — stores markdown files, notes,
/// scripts, and working documents in RocksDB.
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

    /// Save all files to RocksDB
    pub fn save(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string(&self.files)?;
        db.put(CF_WORLD_STATE, "vfs_files", &data)?;
        Ok(())
    }

    /// Load from RocksDB
    pub fn load(db: &Database) -> Result<Self, Box<dyn std::error::Error>> {
        let files: HashMap<String, VfsFile> = db
            .get::<String>(CF_WORLD_STATE, "vfs_files")?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
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
}
