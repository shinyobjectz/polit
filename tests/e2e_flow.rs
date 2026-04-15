//! End-to-end flow tests.
//! Tests the core game systems without UI (headless).

use polit::ai::mock::MockProvider;
use polit::ai::agent::Agent;
use polit::ai::context::GameContext;
use polit::ai::{AiProvider, DmMode};
use polit::ai::config::{AiConfig, AiProviderKind};
use polit::ai::factory::{ConfiguredAiProviderBuilder, ConfiguredAiProviderFactory};
use polit::state::GameStateFs;
use tempfile::TempDir;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_character_creation_flow() {
    let tmp = TempDir::new().unwrap();
    let fs = GameStateFs::open(tmp.path()).unwrap();

    // Simulate: player enters name + avatar
    fs.set_character_field("name", "Homer Simpson").unwrap();
    fs.set_character_field("avatar_face", "[••]").unwrap();
    fs.set_character_field("avatar_color", "Cyan").unwrap();

    // Simulate: agent locks fields during conversation
    fs.set_character_field("background", "Nuclear power plant safety inspector").unwrap();
    fs.set_character_field("motivation", "Accidentally ran for office after a donut-related dare").unwrap();
    fs.set_character_field("party", "Democrat").unwrap();
    fs.set_character_field("tone", "absurdist comedy").unwrap();

    // Verify depth
    assert!(fs.character_depth_percent() >= 30, "Should be startable");

    // Verify character.yaml exists and is parseable
    let char = fs.read_character();
    assert_eq!(char.fields.get("name").unwrap(), "Homer Simpson");
    assert_eq!(char.fields.get("party").unwrap(), "Democrat");
    assert_eq!(char.fields.get("tone").unwrap(), "absurdist comedy");
}

#[derive(Debug)]
struct TaggedProvider {
    name: &'static str,
}

impl TaggedProvider {
    fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl AiProvider for TaggedProvider {
    fn generate(
        &mut self,
        _prompt: &str,
        _mode: DmMode,
    ) -> Result<polit::ai::tools::DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(polit::ai::tools::DmResponse {
            narration: String::new(),
            tool_calls: vec![],
        })
    }

    fn name(&self) -> &str {
        self.name
    }
}

#[derive(Debug, Default)]
struct FakeConfiguredBuilder {
    codex_builds: AtomicUsize,
    openrouter_builds: AtomicUsize,
}

impl ConfiguredAiProviderBuilder for FakeConfiguredBuilder {
    fn build_codex_provider(
        &self,
        _config: &AiConfig,
    ) -> Result<Box<dyn AiProvider>, Box<dyn std::error::Error + Send + Sync>> {
        self.codex_builds.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(TaggedProvider::new("codex")))
    }

    fn build_openrouter_provider(
        &self,
        _config: &AiConfig,
        _storage: &dyn polit::ai::secrets::SecureStorage,
    ) -> Result<Box<dyn AiProvider>, Box<dyn std::error::Error + Send + Sync>> {
        self.openrouter_builds.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(TaggedProvider::new("openrouter")))
    }
}

#[test]
fn valid_ai_config_uses_configured_provider_in_runtime_startup() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("ai.toml");
    let config = AiConfig {
        provider: AiProviderKind::OpenRouter,
        model: Some("openrouter/deepseek-r1".to_string()),
        openrouter_api_key: None,
    };
    config.save(&config_path).unwrap();

    let storage = Box::new(polit::ai::secrets::InMemorySecureStorage::new());
    let builder = Box::new(FakeConfiguredBuilder::default());
    let factory = ConfiguredAiProviderFactory::with_parts(config_path, storage, builder);

    let provider = factory.build_provider_for_runtime().unwrap();

    assert_eq!(provider.name(), "openrouter");
    assert_ne!(provider.name(), "mock-dm");
}

#[test]
fn character_creation_and_game_use_the_same_configured_provider_kind() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("ai.toml");
    let config = AiConfig {
        provider: AiProviderKind::Codex,
        model: None,
        openrouter_api_key: None,
    };
    config.save(&config_path).unwrap();

    let storage = Box::new(polit::ai::secrets::InMemorySecureStorage::new());
    let builder = Box::new(FakeConfiguredBuilder::default());
    let factory = ConfiguredAiProviderFactory::with_parts(config_path, storage, builder);

    let character_provider = factory.build_provider_for_character_creation().unwrap();
    let runtime_provider = factory.build_provider_for_runtime().unwrap();

    assert_eq!(character_provider.name(), "codex");
    assert_eq!(runtime_provider.name(), "codex");
    assert_eq!(character_provider.name(), runtime_provider.name());
}

#[test]
fn test_agent_produces_tool_calls_with_mock() {
    let mut agent = Agent::new(DmMode::CharacterCreation);
    let mut provider = MockProvider::new();
    let context = GameContext::default();

    let response = agent.run_turn(
        "I was a lawyer before going into politics",
        &context,
        &mut provider,
        |_tool| Some("ok".into()),
        None,
    );

    assert!(!response.narration.is_empty(), "Should produce narration");
    // Mock provider should produce tool calls in CharacterCreation mode
    assert!(!response.tool_calls.is_empty(), "Should produce tool calls");
}

#[test]
fn test_agent_memory_persists_across_turns() {
    let mut agent = Agent::new(DmMode::CharacterCreation);
    let mut provider = MockProvider::new();
    let context = GameContext::default();

    // Turn 1
    let r1 = agent.run_turn(
        "My name is Homer",
        &context,
        &mut provider,
        |_| Some("ok".into()),
        None,
    );
    assert_eq!(agent.memory.turn_count(), 1);

    // Turn 2 — should have history from turn 1
    let r2 = agent.run_turn(
        "I work at a nuclear plant",
        &context,
        &mut provider,
        |_| Some("ok".into()),
        None,
    );
    assert_eq!(agent.memory.turn_count(), 2);

    // Verify history block includes both turns
    let history = agent.memory.build_history_block();
    assert!(history.contains("Homer"), "History should contain turn 1 input");
}

#[test]
fn test_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let current = tmp.path().join("current");
    let backup = tmp.path().join("backup_save");

    // Create a save
    let fs = GameStateFs::open(&current).unwrap();
    fs.set_character_field("name", "Fred Flintstone").unwrap();
    fs.write_world(&polit::state::gamestate_fs::WorldFile {
        week: 5,
        year: 2024,
        phase: "Action".into(),
        ap_current: 3,
        ap_max: 5,
        scenario: "modern_usa".into(),
        difficulty: "normal".into(),
    }).unwrap();

    // Save it
    fs.save_as(&backup).unwrap();

    // Verify backup has the data
    let loaded = GameStateFs::open(&backup).unwrap();
    assert_eq!(loaded.get_character_field("name").unwrap(), "Fred Flintstone");
    let world = loaded.read_world();
    assert_eq!(world.week, 5);
    assert_eq!(world.ap_current, 3);
}

#[test]
fn test_conversation_memory_yaml_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let fs = GameStateFs::open(tmp.path()).unwrap();

    // Add exchanges
    fs.add_exchange(1, "hello", "Hi there!", &[]).unwrap();
    fs.add_exchange(2, "who am I?", "You're Homer.", &["lock_field: name".into()]).unwrap();

    // Read back
    let conv = fs.read_conversation();
    assert_eq!(conv.exchanges.len(), 2);
    assert_eq!(conv.exchanges[0].player, "hello");
    assert_eq!(conv.exchanges[1].tools, vec!["lock_field: name".to_string()]);
}

#[test]
fn test_npc_index_stays_in_sync() {
    let tmp = TempDir::new().unwrap();
    let fs = GameStateFs::open(tmp.path()).unwrap();

    let npc = polit::state::gamestate_fs::NpcFile {
        entry: polit::state::gamestate_fs::NpcEntry {
            id: "davis".into(),
            name: "Councilwoman Davis".into(),
            role: "Rival".into(),
            mood: "hostile".into(),
            trust: -20,
            respect: 30,
        },
        personality: "Ambitious".into(),
        memories: vec!["Blocked zoning proposal".into()],
        goals: vec![],
    };
    fs.write_npc(&npc).unwrap();

    // Add another NPC
    let npc2 = polit::state::gamestate_fs::NpcFile {
        entry: polit::state::gamestate_fs::NpcEntry {
            id: "chen".into(),
            name: "Gov. Chen".into(),
            role: "Ally".into(),
            mood: "neutral".into(),
            trust: 40,
            respect: 60,
        },
        ..Default::default()
    };
    fs.write_npc(&npc2).unwrap();

    // Index should have both
    let index = fs.read_npc_index();
    assert_eq!(index.npcs.len(), 2);
    assert!(index.npcs.iter().any(|n| n.id == "davis"));
    assert!(index.npcs.iter().any(|n| n.id == "chen"));
}

#[test]
fn test_native_format_parses_execute_tool() {
    let raw = r#"<execute_tool>
{"name": "lock_field", "field": "background", "value": "former prosecutor"}
</execute_tool>

That's fascinating! What drove you from the courtroom into politics?"#;

    let parsed = polit::ai::native_format::parse_response(raw);
    assert_eq!(parsed.tool_calls.len(), 1, "Should parse execute_tool");
    assert!(parsed.narration.contains("fascinating"), "Should extract narration");
}

#[test]
fn test_native_format_parses_channel_reasoning() {
    let raw = r#"<|channel>thought
Player mentioned being a prosecutor. Should lock background.
<channel|>
<|tool_call>call:lock_field{field:<|"|>background<|"|>,value:<|"|>former prosecutor<|"|>}<tool_call|>
A prosecutor! What made you leave?"#;

    let parsed = polit::ai::native_format::parse_response(raw);
    assert!(parsed.reasoning.is_some(), "Should extract reasoning");
    assert!(parsed.reasoning.unwrap().contains("prosecutor"));
    assert!(!parsed.tool_calls.is_empty(), "Should parse tool call");
    assert!(parsed.narration.contains("prosecutor"), "Should extract narration");
}

#[test]
fn test_field_append_not_overwrite() {
    let tmp = TempDir::new().unwrap();
    let fs = GameStateFs::open(tmp.path()).unwrap();

    fs.set_character_field("traits", "lazy").unwrap();
    fs.append_character_field("traits", "funny").unwrap();
    fs.append_character_field("traits", "lazy").unwrap(); // duplicate — should not re-add

    let val = fs.get_character_field("traits").unwrap();
    assert!(val.contains("lazy"));
    assert!(val.contains("funny"));
    // Should only appear once (no duplicate "lazy; lazy")
    assert_eq!(val.matches("lazy").count(), 1, "Should not duplicate");
}

#[test]
fn test_full_save_directory_structure() {
    let tmp = TempDir::new().unwrap();
    let fs = GameStateFs::open(tmp.path()).unwrap();

    fs.set_character_field("name", "Test").unwrap();
    fs.write_world(&Default::default()).unwrap();
    fs.write_tone(&polit::state::gamestate_fs::ToneFile {
        style: "dark comedy".into(),
        description: "".into(),
    }).unwrap();
    fs.add_exchange(1, "hi", "hello", &[]).unwrap();
    fs.write_notebook("agent notes here").unwrap();

    let files = fs.list_files();
    assert!(files.contains(&"character.yaml".to_string()));
    assert!(files.contains(&"world.yaml".to_string()));
    assert!(files.contains(&"tone.yaml".to_string()));
    assert!(files.iter().any(|f| f.contains("conversation.yaml")));
    assert!(files.iter().any(|f| f.contains("notebook.md")));
}
