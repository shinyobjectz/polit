use serde::{Deserialize, Serialize};

use crate::persistence::{Database, CF_WORLD_STATE};

/// A single conversation exchange (user + assistant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub turn: u32,
    pub user_input: String,
    pub assistant_response: String,
    pub tool_calls_summary: Vec<String>, // e.g., ["modify_rel: davis trust +5", "narrate"]
    pub timestamp_week: u32,
}

/// Conversation memory stored in RocksDB
/// Manages rolling context window with automatic summarization
pub struct ConversationMemory {
    /// Recent exchanges kept in full
    recent: Vec<Exchange>,
    /// Summarized older context
    summary: String,
    /// Max recent exchanges before summarization
    max_recent: usize,
    /// Current turn counter
    turn_counter: u32,
}

impl ConversationMemory {
    pub fn new(max_recent: usize) -> Self {
        Self {
            recent: Vec::new(),
            summary: String::new(),
            max_recent,
            turn_counter: 0,
        }
    }

    /// Add a new exchange
    pub fn add_exchange(
        &mut self,
        user_input: &str,
        assistant_response: &str,
        tool_calls: &[String],
        week: u32,
    ) {
        self.turn_counter += 1;
        self.recent.push(Exchange {
            turn: self.turn_counter,
            user_input: user_input.to_string(),
            assistant_response: assistant_response.to_string(),
            tool_calls_summary: tool_calls.to_vec(),
            timestamp_week: week,
        });

        // If we exceed max_recent, compress oldest exchanges into summary
        if self.recent.len() > self.max_recent {
            self.compress();
        }
    }

    /// Compress oldest exchanges into the summary
    fn compress(&mut self) {
        let to_compress = self.recent.len() - self.max_recent;
        if to_compress == 0 {
            return;
        }

        let old: Vec<Exchange> = self.recent.drain(..to_compress).collect();

        // Build summary of compressed exchanges
        let mut new_summary_parts = Vec::new();
        for ex in &old {
            let tools = if ex.tool_calls_summary.is_empty() {
                String::new()
            } else {
                format!(" [{}]", ex.tool_calls_summary.join(", "))
            };
            new_summary_parts.push(format!(
                "Turn {}: Player said '{}' → DM responded about {}.{}",
                ex.turn,
                truncate(&ex.user_input, 50),
                truncate(&ex.assistant_response, 80),
                tools,
            ));
        }

        if self.summary.is_empty() {
            self.summary = format!("Previous conversation:\n{}", new_summary_parts.join("\n"));
        } else {
            self.summary = format!("{}\n{}", self.summary, new_summary_parts.join("\n"));
        }

        // Keep summary under ~500 chars
        if self.summary.len() > 500 {
            let lines: Vec<&str> = self.summary.lines().collect();
            let keep = lines.len() / 2; // keep recent half
            self.summary = lines[lines.len() - keep..].join("\n");
        }
    }

    /// Build the conversation history block for the prompt
    pub fn build_history_block(&self) -> String {
        let mut parts = Vec::new();

        if !self.summary.is_empty() {
            parts.push(self.summary.clone());
            parts.push(String::new());
        }

        for ex in &self.recent {
            parts.push(format!("Player: {}", ex.user_input));
            parts.push(format!("DM: {}", truncate(&ex.assistant_response, 200)));
            if !ex.tool_calls_summary.is_empty() {
                parts.push(format!("  [Actions: {}]", ex.tool_calls_summary.join(", ")));
            }
            parts.push(String::new());
        }

        parts.join("\n")
    }

    /// Estimate token count (rough: 1 token ≈ 4 chars)
    pub fn estimated_tokens(&self) -> usize {
        let history = self.build_history_block();
        history.len() / 4
    }

    /// Save to RocksDB
    pub fn save(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_string(&self.recent)?;
        db.put(CF_WORLD_STATE, "conversation_recent", &data)?;
        db.put(CF_WORLD_STATE, "conversation_summary", &self.summary)?;
        db.put(CF_WORLD_STATE, "conversation_turn", &self.turn_counter)?;
        Ok(())
    }

    /// Load from RocksDB
    pub fn load(db: &Database, max_recent: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let recent: Vec<Exchange> = db
            .get::<String>(CF_WORLD_STATE, "conversation_recent")?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let summary: String = db
            .get::<String>(CF_WORLD_STATE, "conversation_summary")?
            .unwrap_or_default();

        let turn_counter: u32 = db.get(CF_WORLD_STATE, "conversation_turn")?.unwrap_or(0);

        Ok(Self {
            recent,
            summary,
            max_recent,
            turn_counter,
        })
    }

    pub fn turn_count(&self) -> u32 {
        self.turn_counter
    }

    pub fn recent_count(&self) -> usize {
        self.recent.len()
    }

    /// Clear all memory (new game)
    pub fn clear(&mut self) {
        self.recent.clear();
        self.summary.clear();
        self.turn_counter = 0;
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_exchange() {
        let mut mem = ConversationMemory::new(5);
        mem.add_exchange("hello", "Hi there!", &[], 1);
        assert_eq!(mem.turn_count(), 1);
        assert_eq!(mem.recent_count(), 1);
    }

    #[test]
    fn test_compression() {
        let mut mem = ConversationMemory::new(3);
        for i in 0..6 {
            mem.add_exchange(
                &format!("question {}", i),
                &format!("answer {}", i),
                &[],
                i as u32,
            );
        }
        assert_eq!(mem.recent_count(), 3); // only 3 kept
        assert!(!mem.summary.is_empty()); // older ones summarized
    }

    #[test]
    fn test_history_block() {
        let mut mem = ConversationMemory::new(5);
        mem.add_exchange("what's my name", "Your name is Alex", &[], 1);
        mem.add_exchange(
            "meet Davis",
            "You sit down with Davis",
            &["modify_rel: trust +5".into()],
            1,
        );

        let block = mem.build_history_block();
        assert!(block.contains("what's my name"));
        assert!(block.contains("meet Davis"));
        assert!(block.contains("modify_rel"));
    }

    #[test]
    fn test_token_estimation() {
        let mut mem = ConversationMemory::new(5);
        mem.add_exchange("hello world", "Hi there, welcome to the game!", &[], 1);
        assert!(mem.estimated_tokens() > 0);
        assert!(mem.estimated_tokens() < 100);
    }

    #[test]
    fn test_clear() {
        let mut mem = ConversationMemory::new(5);
        mem.add_exchange("test", "response", &[], 1);
        mem.clear();
        assert_eq!(mem.turn_count(), 0);
        assert_eq!(mem.recent_count(), 0);
    }
}
