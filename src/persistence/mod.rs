use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// Column family names matching GDD spec
pub const CF_CHARACTERS: &str = "characters";
pub const CF_RELATIONSHIPS: &str = "relationships";
pub const CF_NPC_MEMORIES: &str = "npc_memories";
pub const CF_LAWS: &str = "laws";
pub const CF_ECONOMY: &str = "economy";
pub const CF_CARDS: &str = "cards";
pub const CF_WORLD_STATE: &str = "world_state";
pub const CF_INFORMATION: &str = "information";
pub const CF_META_PROGRESSION: &str = "meta_progression";
pub const CF_CUSTOM_EVENTS: &str = "custom_events";
pub const CF_WIKI_CACHE: &str = "wiki_cache";

const ALL_CFS: &[&str] = &[
    CF_CHARACTERS,
    CF_RELATIONSHIPS,
    CF_NPC_MEMORIES,
    CF_LAWS,
    CF_ECONOMY,
    CF_CARDS,
    CF_WORLD_STATE,
    CF_INFORMATION,
    CF_META_PROGRESSION,
    CF_CUSTOM_EVENTS,
    CF_WIKI_CACHE,
];

/// Database wrapper around RocksDB
pub struct Database {
    db: DB,
    path: PathBuf,
}

impl Database {
    /// Open or create the database with all column families
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = PathBuf::from(path);

        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let cf_descriptors: Vec<ColumnFamilyDescriptor> = ALL_CFS
            .iter()
            .map(|name| {
                let cf_opts = Options::default();
                ColumnFamilyDescriptor::new(*name, cf_opts)
            })
            .collect();

        let db = DB::open_cf_descriptors(&db_opts, &path, cf_descriptors)?;

        Ok(Self { db, path })
    }

    /// Put a serializable value into a column family
    pub fn put<T: Serialize>(
        &self,
        cf_name: &str,
        key: &str,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cf = self.db.cf_handle(cf_name)
            .ok_or_else(|| format!("Column family '{}' not found", cf_name))?;
        let bytes = serde_json::to_vec(value)?;
        self.db.put_cf(&cf, key.as_bytes(), &bytes)?;
        Ok(())
    }

    /// Get a deserializable value from a column family
    pub fn get<T: DeserializeOwned>(
        &self,
        cf_name: &str,
        key: &str,
    ) -> Result<Option<T>, Box<dyn std::error::Error>> {
        let cf = self.db.cf_handle(cf_name)
            .ok_or_else(|| format!("Column family '{}' not found", cf_name))?;
        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(bytes) => {
                let value: T = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Delete a key from a column family
    pub fn delete(
        &self,
        cf_name: &str,
        key: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cf = self.db.cf_handle(cf_name)
            .ok_or_else(|| format!("Column family '{}' not found", cf_name))?;
        self.db.delete_cf(&cf, key.as_bytes())?;
        Ok(())
    }

    /// Scan all keys with a given prefix in a column family
    pub fn scan_prefix<T: DeserializeOwned>(
        &self,
        cf_name: &str,
        prefix: &str,
    ) -> Result<Vec<(String, T)>, Box<dyn std::error::Error>> {
        let cf = self.db.cf_handle(cf_name)
            .ok_or_else(|| format!("Column family '{}' not found", cf_name))?;

        let mut results = Vec::new();
        let iter = self.db.prefix_iterator_cf(&cf, prefix.as_bytes());

        for item in iter {
            let (key_bytes, value_bytes) = item?;
            let key = String::from_utf8(key_bytes.to_vec())?;
            if !key.starts_with(prefix) {
                break;
            }
            let value: T = serde_json::from_slice(&value_bytes)?;
            results.push((key, value));
        }

        Ok(results)
    }

    /// Create a snapshot (checkpoint) for save system
    pub fn create_snapshot(&self, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let snapshot_path = self.path.parent()
            .unwrap_or(Path::new("."))
            .join("saves")
            .join(name);

        // Checkpoint requires a non-existent target directory
        if snapshot_path.exists() {
            std::fs::remove_dir_all(&snapshot_path)?;
        }
        if let Some(parent) = snapshot_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let checkpoint = rocksdb::checkpoint::Checkpoint::new(&self.db)?;
        checkpoint.create_checkpoint(&snapshot_path)?;

        Ok(snapshot_path)
    }

    /// List all column family names
    pub fn column_families() -> &'static [&'static str] {
        ALL_CFS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::components::Relationship;

    #[test]
    fn test_open_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap());
        assert!(db.is_ok());
    }

    #[test]
    fn test_put_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap()).unwrap();

        let rel = Relationship::default();
        db.put(CF_RELATIONSHIPS, "player:npc1", &rel).unwrap();

        let retrieved: Option<Relationship> = db.get(CF_RELATIONSHIPS, "player:npc1").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.trust, 0);
        assert_eq!(retrieved.respect, 0);
    }

    #[test]
    fn test_get_missing_key() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap()).unwrap();

        let result: Option<Relationship> = db.get(CF_RELATIONSHIPS, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap()).unwrap();

        let rel = Relationship::default();
        db.put(CF_RELATIONSHIPS, "player:npc1", &rel).unwrap();
        db.delete(CF_RELATIONSHIPS, "player:npc1").unwrap();

        let result: Option<Relationship> = db.get(CF_RELATIONSHIPS, "player:npc1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_scan_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap()).unwrap();

        let rel1 = Relationship { trust: 10, ..Relationship::default() };
        let rel2 = Relationship { trust: 20, ..Relationship::default() };
        let rel3 = Relationship { trust: 30, ..Relationship::default() };

        db.put(CF_RELATIONSHIPS, "player:npc1", &rel1).unwrap();
        db.put(CF_RELATIONSHIPS, "player:npc2", &rel2).unwrap();
        db.put(CF_RELATIONSHIPS, "other:npc3", &rel3).unwrap();

        let results: Vec<(String, Relationship)> =
            db.scan_prefix(CF_RELATIONSHIPS, "player:").unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_all_column_families_accessible() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap()).unwrap();

        for cf in ALL_CFS {
            db.put(cf, "test_key", &"test_value".to_string()).unwrap();
            let val: Option<String> = db.get(cf, "test_key").unwrap();
            assert!(val.is_some(), "Failed to read from CF: {}", cf);
        }
    }

    #[test]
    fn test_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_str().unwrap()).unwrap();

        db.put(CF_WORLD_STATE, "week", &42u32).unwrap();

        let snap_path = db.create_snapshot("test_save").unwrap();
        assert!(snap_path.exists());
    }
}
