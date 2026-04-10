# POLIT — The American Politics Simulator

## Project Overview

CLI-based political RPG built in Rust (Ratatui + Crossterm), powered by local Gemma 4 AI dungeon master (Candle/ONNX). Roguelike with meta-progression, deckbuilder mechanics, real economic data seeding, and fully moddable scenario SDK.

## Game Design Document Reference

All design docs live in `docs/gdd/`. Each section has frontmatter with `depends_on` and `blocks` fields for dependency tracking.

| Section | Path | Key Topics |
|---------|------|-----------|
| [Core Architecture](docs/gdd/01-core-architecture/README.md) | `01-core-architecture/` | bevy_ecs, event bus, RocksDB column families, project structure, async threads |
| [Game Loop](docs/gdd/02-game-loop/README.md) | `02-game-loop/` | Dawn/Action/Event/Dusk phases, AP economy, D20 dice, phase state machine |
| [Deckbuilder](docs/gdd/03-deckbuilder/README.md) | `03-deckbuilder/` | Tactic/Asset/Position cards, flip-flop coherence, card evolution, rarity |
| [AI Harness](docs/gdd/04-ai-harness/README.md) | `04-ai-harness/` | Gemma via ort (ONNX Runtime), 15 DM tools, GBNF grammar, context budget (~10k tokens), editable prompts |
| [Economic Simulation](docs/gdd/05-economic-simulation/README.md) | `05-economic-simulation/` | 4-layer model (surface→macro→demographic→geopolitical), policy causality chains |
| [NPC & Social Graph](docs/gdd/06-npc-social-graph/README.md) | `06-npc-social-graph/` | petgraph network, Big Five personality, reputation propagation, staff system, family |
| [Law Engine](docs/gdd/07-law-engine/README.md) | `07-law-engine/` | 5 enforcement types, legislative process, constitutional supremacy, RAG-based interpretation |
| [Character & Meta-Progression](docs/gdd/08-character-meta-progression/README.md) | `08-character-meta-progression/` | 9+ archetypes, legacy points, Hall of Fame, run scoring (S-F), difficulty modes |
| [UI Design](docs/gdd/09-ui-design/README.md) | `09-ui-design/` | Chat-forward interface, floating overlays, phase-aware status bar, slash commands |
| [Freeform Action Engine](docs/gdd/10-freeform-action-engine/README.md) | `10-freeform-action-engine/` | Custom events, DM improvisation, self-extending SDK, Rhai script generation |
| [Modding & SDK](docs/gdd/11-modding-sdk/README.md) | `11-modding-sdk/` | 6 mod types, 40+ hook points, Rhai API, SDK CLI, total conversion support |
| [Data Pipeline](docs/gdd/12-data-pipeline/README.md) | `12-data-pipeline/` | FRED/Census/BLS/BEA/Wikipedia APIs, 5 game start modes, fact-checking system |
| [Audio](docs/gdd/13-audio/README.md) | `13-audio/` | Voice input via whisper-rs STT, optional TTS, speech gameplay modifiers |
| [Game Design Principles](docs/gdd/14-game-design-principles/README.md) | `14-game-design-principles/` | 6 pillars, emergent scenarios, roguelike DNA, anti-stagnation systems |
| [News & Information](docs/gdd/15-news-information/README.md) | `15-news-information/` | Information entities, knowledge graph, media orgs, news cycles, cross-system effects |
| [Corporate System](docs/gdd/16-corporate-system/README.md) | `16-corporate-system/` | Sector interest tables, action/reaction matrix, lobbying, campaign finance, foreign influence |
| [Geopolitics](docs/gdd/17-geopolitics/README.md) | `17-geopolitics/` | Foreign powers (3 tiers), war system, nuclear arsenal, covert ops, proxy wars, espionage |
| [Government Departments](docs/gdd/18-government-departments/README.md) | `18-government-departments/` | Federal hierarchy, 6 career tracks, bureaucratic gameplay, department data seeding |
| [Elections](docs/gdd/19-elections/README.md) | `19-elections/` | Primary/general arcs, vote calculation formula, Electoral College, campaign map |
| [Tutorial & Onboarding](docs/gdd/20-tutorial-onboarding/README.md) | `20-tutorial-onboarding/` | 12-week guided campaign, progressive disclosure, contextual help |
| [Performance & CI/CD](docs/gdd/21-performance-cicd/README.md) | `21-performance-cicd/` | 3 hardware tiers, inference budget, async architecture, test categories, release pipeline |

## Design Pillars (always enforce)

1. **Everything is systems, not scripts** — emergent outcomes from interacting simulations, never hard-coded drama
2. **Consequences are real and permanent** — social graph has memory, economy has inertia, laws persist
3. **Every playthrough is unique** — procedural generation ensures no two runs alike
4. **Depth without complexity walls** — chat-forward UI, 5-minute onboarding, depth reveals through play
5. **AI is a dungeon master, not game designer** — narrates and adjudicates, never overrides systems
6. **Failure is content** — losing is a story, meta-progression rewards all endings

## Key Technical Decisions

- **Engine**: bevy_ecs (without renderer) + plugin-style extensibility
- **AI**: Gemma 4 E4B/E2B via ort (ONNX Runtime) with Q4 quantized ONNX models from onnx-community. Set HF_TOKEN env var for model download.
- **Database**: RocksDB with column families per domain, snapshot-based saves
- **UI**: Ratatui + Crossterm, chat-forward with floating overlays
- **Scripting**: Rhai (sandboxed, for moddable game logic)
- **Graph**: petgraph for social network computation
- **Audio**: cpal for I/O, whisper-rs for local STT (speech-to-text), optional system TTS

## Code Conventions

- Rust 2021 edition
- `clippy::pedantic` lint level
- `rustfmt` with default settings
- ECS components as plain structs with `#[derive(Component)]`
- Systems as standalone functions
- All game data in TOML (human-readable, moddable)
- Prompts in `game/prompts/` — always editable, never hardcoded
- Column family per domain in RocksDB

## Development Commands

```bash
cargo run                        # launch game (debug)
cargo run -- --headless          # headless simulation
cargo run -- --mock-ai           # deterministic AI for testing
cargo run -- --tutorial          # jump to tutorial
cargo test                       # unit + integration tests
cargo test --features sim        # headless simulation tests
cargo bench                      # performance benchmarks
polit-data fetch --all           # refresh real-world data
polit-sdk validate game/         # validate scenario data
```
