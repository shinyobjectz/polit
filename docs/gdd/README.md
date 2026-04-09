# POLIT — Game Design Document

**The American Politics Simulator**

A CLI-based political RPG built with Ratatui/Crossterm in Rust, powered by a local Gemma 4 AI dungeon master running via Candle/ONNX. Roguelike with meta-progression, deckbuilder mechanics, real economic data, and a fully moddable scenario SDK.

## Design Pillars

1. **Everything is systems, not scripts** — emergent outcomes from interacting simulations
2. **Consequences are real and permanent** — the social graph has memory, the economy has inertia
3. **Every playthrough is a unique story** — procedural generation ensures no two runs are alike
4. **Depth without complexity walls** — chat-forward UI, learn by playing, depth reveals itself
5. **The AI is a dungeon master, not a game designer** — narrates and adjudicates, never overrides systems
6. **Failure is content** — losing is a story, not a punishment

## Document Index

| # | Section | Path | Description |
|---|---------|------|-------------|
| 01 | [Core Architecture](01-core-architecture/README.md) | `01-core-architecture/` | ECS engine, event bus, persistence, project structure |
| 02 | [Game Loop](02-game-loop/README.md) | `02-game-loop/` | Turn structure, phases, action points, dice system |
| 03 | [Deckbuilder](03-deckbuilder/README.md) | `03-deckbuilder/` | Card taxonomy, combos, coherence, meta-progression |
| 04 | [AI Harness](04-ai-harness/README.md) | `04-ai-harness/` | Gemma via mistral.rs, tool suite, GBNF grammar, context management |
| 05 | [Economic Simulation](05-economic-simulation/README.md) | `05-economic-simulation/` | Layered macro model, policy causality, tick pipeline |
| 06 | [NPC & Social Graph](06-npc-social-graph/README.md) | `06-npc-social-graph/` | Character entities, petgraph network, propagation, lifecycles |
| 07 | [Law Engine](07-law-engine/README.md) | `07-law-engine/` | Legislative process, enforcement types, constitutional system |
| 08 | [Character & Meta-Progression](08-character-meta-progression/README.md) | `08-character-meta-progression/` | Creation flow, archetypes, legacy system, Hall of Fame |
| 09 | [UI Design](09-ui-design/README.md) | `09-ui-design/` | Chat-forward interface, overlays, phase-aware rendering |
| 10 | [Freeform Action Engine](10-freeform-action-engine/README.md) | `10-freeform-action-engine/` | Custom events, DM improvisation, self-extending SDK |
| 11 | [Modding & SDK](11-modding-sdk/README.md) | `11-modding-sdk/` | Mod types, hook points, scenario SDK, Rhai scripting API |
| 12 | [Data Pipeline](12-data-pipeline/README.md) | `12-data-pipeline/` | Real-world data ingestion, APIs, seeding by game mode |
| 13 | [Audio](13-audio/README.md) | `13-audio/` | Voice input, TTS output, speech gameplay |
| 14 | [Game Design Principles](14-game-design-principles/README.md) | `14-game-design-principles/` | Emergent play, roguelike DNA, anti-stagnation |
| 15 | [News & Information](15-news-information/README.md) | `15-news-information/` | Information entities, media ecosystem, news cycles |
| 16 | [Corporate System](16-corporate-system/README.md) | `16-corporate-system/` | Sector lookup tables, lobbying, foreign influence |
| 17 | [Geopolitics](17-geopolitics/README.md) | `17-geopolitics/` | Foreign powers, war, nuclear arsenal, covert ops |
| 18 | [Government Departments](18-government-departments/README.md) | `18-government-departments/` | Bureaucratic careers, department gameplay, data seeding |
| 19 | [Elections](19-elections/README.md) | `19-elections/` | Primary/general campaigns, Electoral College, vote math |
| 20 | [Tutorial & Onboarding](20-tutorial-onboarding/README.md) | `20-tutorial-onboarding/` | First Day in Office, progressive disclosure, help system |
| 21 | [Performance & CI/CD](21-performance-cicd/README.md) | `21-performance-cicd/` | Hardware tiers, async architecture, build pipeline |

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Engine | bevy_ecs (no renderer) + plugin elements | Composable systems, parallelism, moddability |
| AI | Gemma 12B-it via mistral.rs (local GGUF) | Privacy, offline play, built-in tool calling, grammar-constrained output |
| Database | RocksDB | Embedded, fast, column families per domain, snapshots for saves |
| UI Framework | Ratatui + Crossterm | Mature Rust TUI, cross-platform, sufficient for chat-forward design |
| Scripting | Rhai | Lightweight, sandboxed, Rust-native, good for moddable game logic |
| Graph | petgraph | Rust-native graph library for social network computation |
| Audio | cpal + whisper-rs (local STT) | Minimal dependencies, proven speech-to-text pipeline |

## Era Support

| Mode | Data Source | Description |
|------|------------|-------------|
| Modern (2024+) | Live API fetch | Real current politicians, economy, geopolitics |
| Historical | Historical API data | Any point in US history with period-accurate conditions |
| Alternate History | Historical fork + AI extrapolation | Diverge from a real historical point |
| Speculative | Trend projection + parameters | Plausible future scenarios |
| Fictional | Scenario TOML files | Fully custom worlds, total conversion mods |
