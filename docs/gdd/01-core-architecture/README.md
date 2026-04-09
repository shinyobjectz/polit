---
title: Core Architecture
section: 01
status: design-complete
depends_on: []
blocks: [02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13]
---

# Core Architecture

## Overview

POLIT uses a **bevy_ecs** (v0.18, standalone — no full Bevy renderer needed) Entity Component System as the simulation core, with plugin-style extensibility for modding. Everything in the game world — characters, districts, laws, economic sectors, cards, information, corporations, foreign nations — is an ECS entity with components. Systems process them each game tick.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    POLIT Engine                          │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ Ratatui  │  │  Event   │  │  Rhai    │              │
│  │ Renderer │◄─┤   Bus    ├──►│ Scripting│              │
│  └────▲─────┘  └────┬─────┘  └──────────┘              │
│       │              │                                   │
│  ┌────┴──────────────┴──────────────────┐               │
│  │          bevy_ecs World              │               │
│  │                                      │               │
│  │  ┌─────────┐ ┌─────────┐ ┌────────┐ │               │
│  │  │Political│ │Economic │ │  NPC   │ │               │
│  │  │ Systems │ │ Systems │ │Systems │ │               │
│  │  └─────────┘ └─────────┘ └────────┘ │               │
│  │  ┌─────────┐ ┌─────────┐ ┌────────┐ │               │
│  │  │  Card   │ │  Law    │ │ Event  │ │               │
│  │  │ Systems │ │ Engine  │ │Generator│ │               │
│  │  └─────────┘ └─────────┘ └────────┘ │               │
│  └──────────────────┬───────────────────┘               │
│                     │                                    │
│  ┌──────────────────┴───────────────────┐               │
│  │         AI Harness (Gemma)           │               │
│  │  mistral.rs inference · Tool router  │               │
│  │  GBNF grammar · Context builder      │               │
│  └──────────────────┬───────────────────┘               │
│                     │                                    │
│  ┌──────────────────┴───────────────────┐               │
│  │        RocksDB Persistence           │               │
│  │  Column families per domain          │               │
│  │  Snapshot/restore for save system    │               │
│  └──────────────────────────────────────┘               │
└─────────────────────────────────────────────────────────┘
```

## Key Architectural Decisions

### bevy_ecs as Simulation Core

Entities represent all game objects. Systems run each tick to advance the simulation. This provides:

- **Composability**: New mechanics = new components + systems
- **Parallelism**: Independent systems run concurrently
- **Decoupling**: Social graph, economy, and politics are independent systems operating on shared entities
- **Moddability**: Mods add new components and systems via data files + Rhai scripts

### Event Bus

A typed event channel bridges ECS, UI, and AI:

- Game events (election called, scandal breaks, bill introduced) flow through the bus
- AI harness subscribes to events needing narrative generation
- UI subscribes to events needing rendering
- Mods can subscribe to any event via hooks

### Rhai Scripting

First-party systems are Rust for performance. Mod systems use Rhai (sandboxed):

- Event chains, custom card effects, win conditions, scenario setup
- Script API exposes safe game operations (see [Modding & SDK](../11-modding-sdk/README.md))
- Scripts validated before execution, size-limited, sandboxed

### RocksDB Persistence

Column families separate game domains:

| Column Family | Contents |
|--------------|----------|
| `characters` | NPC and player character entities |
| `relationships` | Social graph edges |
| `npc_memories` | Per-character memory entries |
| `laws` | Active and historical legislation |
| `economy` | Economic state variables |
| `cards` | Card definitions and player deck state |
| `world_state` | Global simulation variables |
| `information` | Information entities and knowledge graph |
| `meta_progression` | Cross-run unlocks, Hall of Fame |
| `custom_events` | DM-generated event schemas |
| `wiki_cache` | Cached Wikipedia/Wikidata for fact-checking |

Snapshot-based saves via RocksDB checkpoint for instant save/restore.

## Project Structure

```
polit/
├─ src/
│  ├─ main.rs
│  ├─ engine/                  ECS world, game loop, tick
│  ├─ systems/                 All ECS systems
│  │  ├─ political.rs          Government simulation
│  │  ├─ economic.rs           Macro model
│  │  ├─ demographic.rs        Population model
│  │  ├─ geopolitical.rs       World affairs
│  │  ├─ npc.rs                Character AI + lifecycle
│  │  ├─ social_graph.rs       petgraph relationship network
│  │  ├─ cards.rs              Deckbuilder mechanics
│  │  ├─ law_engine.rs         Legislation + enforcement
│  │  ├─ dice.rs               Roll system
│  │  ├─ staff.rs              Staff management
│  │  ├─ family.rs             Personal life
│  │  ├─ events.rs             Event generation + custom events
│  │  ├─ news.rs               Information + media system
│  │  ├─ corporate.rs          Corporate reaction system
│  │  └─ meta.rs               Cross-run progression
│  ├─ ai/                      Gemma 4 harness
│  │  ├─ harness.rs            mistral.rs inference wrapper
│  │  ├─ context_builder.rs    World state → prompt
│  │  ├─ tool_router.rs        Parse AI tool calls → ECS commands
│  │  ├─ custom_action.rs      Freeform action pipeline
│  │  └─ summarizer.rs         State compression for context
│  ├─ ui/                      Ratatui frontend
│  │  ├─ app.rs                Main app state + input handling
│  │  ├─ chat.rs               Narrative stream widget
│  │  ├─ overlays.rs           Floating panels (map, deck, etc.)
│  │  ├─ phases.rs             Phase-aware UI state machine
│  │  ├─ input.rs              Text input + slash commands
│  │  └─ theme.rs              Color/style from config
│  ├─ persistence/             RocksDB layer
│  │  ├─ db.rs                 Column families, snapshots
│  │  ├─ save.rs               Save/load/autosave
│  │  └─ migration.rs          Schema versioning
│  └─ scripting/               Rhai integration
│     ├─ runtime.rs            Sandboxed script execution
│     ├─ api.rs                Game API exposed to scripts
│     └─ loader.rs             Load scripts from scenarios
│
├─ game/                       Data-driven content (moddable)
│  ├─ scenarios/
│  │  ├─ modern_usa/           Default scenario
│  │  │  ├─ scenario.toml
│  │  │  ├─ constitution.toml
│  │  │  ├─ government.toml
│  │  │  ├─ government/        Department data files
│  │  │  ├─ parties.toml
│  │  │  ├─ districts/
│  │  │  ├─ characters/
│  │  │  ├─ events/
│  │  │  ├─ cards/
│  │  │  ├─ economy/
│  │  │  ├─ laws/
│  │  │  ├─ scripts/
│  │  │  └─ history/
│  │  └─ (other scenarios...)
│  ├─ archetypes/              Character starter templates
│  ├─ prompts/                 AI personality (editable)
│  │  ├─ tone.toml
│  │  ├─ dm_system.toml
│  │  ├─ legal_style.toml
│  │  ├─ npc_templates/
│  │  └─ event_templates/
│  └─ config/
│     ├─ balance.toml
│     ├─ theme.toml
│     ├─ audio.toml
│     └─ difficulty.toml
│
├─ sdk/                        Scenario creation toolkit
├─ data/                       Seed data from real sources
├─ saves/                      Player save files
├─ meta/                       Meta-progression state
├─ mods/                       User-installed mods
└─ models/                     Gemma 4 model weights
```

## Async Thread Architecture

| Thread | Responsibility | Framework |
|--------|---------------|-----------|
| UI | Ratatui render loop (60fps) | crossterm event loop |
| Game | ECS tick, simulation systems | bevy_ecs schedule |
| AI | mistral.rs inference (non-blocking) | tokio async |
| IO | RocksDB reads/writes | crossbeam channels |
| Audio | CPAL mic input / TTS output | cpal callbacks |

Communication via crossbeam channels. UI never freezes during AI inference.
