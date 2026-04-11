//! GameStateFs — reads and writes game state as YAML files on disk.
//!
//! Directory layout:
//! ```
//! ~/.polit/saves/<save_name>/
//! ├── character.yaml          # player character fields
//! ├── world.yaml              # week, year, phase, AP
//! ├── tone.yaml               # narrator voice settings
//! ├── npcs/
//! │   ├── index.yaml          # quick-scan summary of all NPCs
//! │   └── <npc_id>.yaml       # full NPC data + memories
//! ├── relationships/
//! │   └── graph.yaml          # all relationship edges
//! ├── economy.yaml            # economic simulation state
//! ├── laws/
//! │   ├── index.yaml          # all laws summary
//! │   └── <law_id>.yaml       # individual law detail
//! ├── cards/
//! │   └── inventory.yaml      # player's card collection
//! ├── memory/
//! │   ├── conversation.yaml   # recent exchanges (rolling window)
//! │   └── summary.md          # compressed older context
//! └── agent/
//!     └── notebook.md         # agent's own scratchpad
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Root handle for all game state file operations.
pub struct GameStateFs {
    root: PathBuf,
}

// ── Character ──────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterFile {
    pub name: String,
    #[serde(default)]
    pub avatar_face: String,
    #[serde(default)]
    pub avatar_color: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, String>,
}

// ── World ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldFile {
    pub week: u32,
    pub year: u32,
    pub phase: String,
    pub ap_current: i32,
    pub ap_max: i32,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub difficulty: String,
}

impl Default for WorldFile {
    fn default() -> Self {
        Self {
            week: 1,
            year: 2024,
            phase: "Character Creation".into(),
            ap_current: 5,
            ap_max: 5,
            scenario: "modern_usa".into(),
            difficulty: "normal".into(),
        }
    }
}

// ── Tone ───────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToneFile {
    pub style: String,
    #[serde(default)]
    pub description: String,
}

// ── NPC ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub trust: i32,
    #[serde(default)]
    pub respect: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NpcIndex {
    #[serde(default)]
    pub npcs: Vec<NpcEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcFile {
    #[serde(flatten)]
    pub entry: NpcEntry,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub memories: Vec<String>,
    #[serde(default)]
    pub goals: Vec<String>,
}

// ── Conversation Memory ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExchange {
    pub turn: u32,
    pub player: String,
    pub narrator: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationFile {
    #[serde(default)]
    pub exchanges: Vec<MemoryExchange>,
}

// ── Implementation ─────────────────────────────────────────

impl GameStateFs {
    /// Open or create a save directory.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        // Create all subdirectories
        for dir in &["npcs", "relationships", "laws", "cards", "memory", "agent"] {
            std::fs::create_dir_all(root.join(dir))?;
        }
        Ok(Self { root })
    }

    /// The root path of this save.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // ── Generic YAML helpers ───────────────────────────────

    fn read_yaml<T: for<'de> Deserialize<'de> + Default>(&self, rel_path: &str) -> T {
        let path = self.root.join(rel_path);
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_yaml::from_str(&content).unwrap_or_default(),
            Err(_) => T::default(),
        }
    }

    fn write_yaml<T: Serialize>(&self, rel_path: &str, data: &T) -> std::io::Result<()> {
        let path = self.root.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(data).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;
        std::fs::write(path, yaml)
    }

    fn read_text(&self, rel_path: &str) -> String {
        let path = self.root.join(rel_path);
        std::fs::read_to_string(path).unwrap_or_default()
    }

    fn write_text(&self, rel_path: &str, content: &str) -> std::io::Result<()> {
        let path = self.root.join(rel_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    // ── Character ──────────────────────────────────────────

    pub fn read_character(&self) -> CharacterFile {
        self.read_yaml("character.yaml")
    }

    pub fn write_character(&self, data: &CharacterFile) -> std::io::Result<()> {
        self.write_yaml("character.yaml", data)
    }

    /// Set a single character field. Reads, updates, writes back.
    pub fn set_character_field(&self, key: &str, value: &str) -> std::io::Result<()> {
        let mut char = self.read_character();
        char.fields.insert(key.to_string(), value.to_string());
        self.write_character(&char)
    }

    /// Append to an existing character field (semicolon-separated).
    pub fn append_character_field(&self, key: &str, value: &str) -> std::io::Result<()> {
        let mut char = self.read_character();
        let existing = char.fields.get(key).cloned().unwrap_or_default();
        if existing.is_empty() {
            char.fields.insert(key.to_string(), value.to_string());
        } else if !existing.contains(value) {
            char.fields.insert(key.to_string(), format!("{}; {}", existing, value));
        }
        self.write_character(&char)
    }

    /// Get a single character field.
    pub fn get_character_field(&self, key: &str) -> Option<String> {
        self.read_character().fields.get(key).cloned()
    }

    /// Get all character fields as (key, value) pairs.
    pub fn character_fields(&self) -> HashMap<String, String> {
        self.read_character().fields
    }

    /// Character depth as percentage (fields filled / total).
    pub fn character_depth_percent(&self) -> u32 {
        let total = 11; // name + 10 discoverable fields
        let filled = self.read_character().fields.len() as u32;
        ((filled as f32 / total as f32) * 100.0).min(100.0) as u32
    }

    // ── World ──────────────────────────────────────────────

    pub fn read_world(&self) -> WorldFile {
        self.read_yaml("world.yaml")
    }

    pub fn write_world(&self, data: &WorldFile) -> std::io::Result<()> {
        self.write_yaml("world.yaml", data)
    }

    // ── Tone ───────────────────────────────────────────────

    pub fn read_tone(&self) -> ToneFile {
        self.read_yaml("tone.yaml")
    }

    pub fn write_tone(&self, data: &ToneFile) -> std::io::Result<()> {
        self.write_yaml("tone.yaml", data)
    }

    // ── NPCs ───────────────────────────────────────────────

    pub fn read_npc_index(&self) -> NpcIndex {
        self.read_yaml("npcs/index.yaml")
    }

    pub fn write_npc_index(&self, data: &NpcIndex) -> std::io::Result<()> {
        self.write_yaml("npcs/index.yaml", data)
    }

    pub fn read_npc(&self, id: &str) -> Option<NpcFile> {
        let path = format!("npcs/{}.yaml", id);
        let file = self.root.join(&path);
        if file.exists() {
            Some(self.read_yaml(&path))
        } else {
            None
        }
    }

    pub fn write_npc(&self, npc: &NpcFile) -> std::io::Result<()> {
        let path = format!("npcs/{}.yaml", npc.entry.id);
        self.write_yaml(&path, npc)?;
        // Update index
        let mut index = self.read_npc_index();
        if let Some(existing) = index.npcs.iter_mut().find(|n| n.id == npc.entry.id) {
            *existing = npc.entry.clone();
        } else {
            index.npcs.push(npc.entry.clone());
        }
        self.write_npc_index(&index)
    }

    // ── Conversation Memory ────────────────────────────────

    pub fn read_conversation(&self) -> ConversationFile {
        self.read_yaml("memory/conversation.yaml")
    }

    pub fn write_conversation(&self, data: &ConversationFile) -> std::io::Result<()> {
        self.write_yaml("memory/conversation.yaml", data)
    }

    pub fn add_exchange(
        &self,
        turn: u32,
        player: &str,
        narrator: &str,
        tools: &[String],
    ) -> std::io::Result<()> {
        let mut conv = self.read_conversation();
        conv.exchanges.push(MemoryExchange {
            turn,
            player: player.to_string(),
            narrator: narrator.to_string(),
            tools: tools.to_vec(),
        });
        // Keep last 6 exchanges in the file
        if conv.exchanges.len() > 6 {
            let overflow: Vec<_> = conv.exchanges.drain(..conv.exchanges.len() - 6).collect();
            // Append overflow to summary
            let summary_lines: Vec<String> = overflow
                .iter()
                .map(|e| format!("Turn {}: {} → {}", e.turn, truncate(&e.player, 50), truncate(&e.narrator, 80)))
                .collect();
            let mut summary = self.read_text("memory/summary.md");
            if !summary.is_empty() {
                summary.push('\n');
            }
            summary.push_str(&summary_lines.join("\n"));
            // Keep summary under ~2000 chars
            if summary.len() > 2000 {
                let lines: Vec<&str> = summary.lines().collect();
                let keep = lines.len() / 2;
                summary = lines[lines.len() - keep..].join("\n");
            }
            self.write_text("memory/summary.md", &summary)?;
        }
        self.write_conversation(&conv)
    }

    pub fn read_summary(&self) -> String {
        self.read_text("memory/summary.md")
    }

    // ── Agent Notebook ─────────────────────────────────────

    pub fn read_notebook(&self) -> String {
        self.read_text("agent/notebook.md")
    }

    pub fn write_notebook(&self, content: &str) -> std::io::Result<()> {
        self.write_text("agent/notebook.md", content)
    }

    // ── Save / Load ────────────────────────────────────────

    /// Copy this save to a new named save.
    pub fn save_as(&self, dest: impl Into<PathBuf>) -> std::io::Result<()> {
        let dest = dest.into();
        copy_dir_recursive(&self.root, &dest)
    }

    /// List all files in this save (for debugging).
    pub fn list_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        list_recursive(&self.root, &self.root, &mut files);
        files.sort();
        files
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

fn list_recursive(base: &Path, current: &Path, out: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                list_recursive(base, &path, out);
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_character_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path()).unwrap();

        fs.set_character_field("background", "former prosecutor").unwrap();
        fs.set_character_field("party", "Democrat").unwrap();

        let char = fs.read_character();
        assert_eq!(char.fields.get("background").unwrap(), "former prosecutor");
        assert_eq!(char.fields.get("party").unwrap(), "Democrat");
        assert_eq!(fs.character_depth_percent(), 18); // 2/11
    }

    #[test]
    fn test_append_field() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path()).unwrap();

        fs.set_character_field("traits", "lazy").unwrap();
        fs.append_character_field("traits", "funny").unwrap();

        let val = fs.get_character_field("traits").unwrap();
        assert!(val.contains("lazy"));
        assert!(val.contains("funny"));
    }

    #[test]
    fn test_world_defaults() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path()).unwrap();

        let world = fs.read_world();
        assert_eq!(world.week, 1);
        assert_eq!(world.year, 2024);
    }

    #[test]
    fn test_conversation_memory() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path()).unwrap();

        for i in 0..8 {
            fs.add_exchange(i, &format!("q{}", i), &format!("a{}", i), &[]).unwrap();
        }

        let conv = fs.read_conversation();
        assert_eq!(conv.exchanges.len(), 6); // capped at 6
        let summary = fs.read_summary();
        assert!(summary.contains("q0")); // older ones in summary
    }

    #[test]
    fn test_npc_with_index() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path()).unwrap();

        let npc = NpcFile {
            entry: NpcEntry {
                id: "davis".into(),
                name: "Councilwoman Davis".into(),
                role: "Rival".into(),
                mood: "hostile".into(),
                trust: -20,
                respect: 30,
            },
            personality: "Ambitious, calculating".into(),
            memories: vec!["Blocked your zoning proposal".into()],
            goals: vec!["Become mayor".into()],
        };
        fs.write_npc(&npc).unwrap();

        let index = fs.read_npc_index();
        assert_eq!(index.npcs.len(), 1);
        assert_eq!(index.npcs[0].name, "Councilwoman Davis");

        let loaded = fs.read_npc("davis").unwrap();
        assert_eq!(loaded.memories[0], "Blocked your zoning proposal");
    }

    #[test]
    fn test_save_as() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path().join("current")).unwrap();

        fs.set_character_field("name", "Homer Simpson").unwrap();
        fs.save_as(tmp.path().join("backup")).unwrap();

        let backup = GameStateFs::open(tmp.path().join("backup")).unwrap();
        assert_eq!(backup.get_character_field("name").unwrap(), "Homer Simpson");
    }

    #[test]
    fn test_list_files() {
        let tmp = TempDir::new().unwrap();
        let fs = GameStateFs::open(tmp.path()).unwrap();
        fs.set_character_field("name", "Test").unwrap();
        fs.write_world(&WorldFile::default()).unwrap();

        let files = fs.list_files();
        assert!(files.iter().any(|f| f == "character.yaml"));
        assert!(files.iter().any(|f| f == "world.yaml"));
    }
}
