---
title: AI Harness & Gemma 4 Integration
section: 04
status: design-complete
depends_on: [01]
blocks: [10, 13]
---

# AI Harness & Gemma 4 Integration

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Context Builder                    │
│  World State → compressed summary of current sim    │
│  Character Info → player stats, cards, relationships│
│  Scene Context → what's happening right now         │
│  NPC Profiles → relevant characters + memories      │
│  Tone Prompt → loaded from editable .toml file      │
│  History Window → recent events + conversation buf  │
└───────────────────────┬─────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────┐
│              Gemma 4 (via Candle)                    │
│  Model: gemma-4-E2B/E4B (edge) or 27B (full)       │
│  Input: text + optional audio                       │
│  Output: structured JSON via tool-calling format    │
└───────────────────────┬─────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────┐
│                   Tool Router                        │
│  Parses AI tool calls → ECS commands                │
└─────────────────────────────────────────────────────┘
```

## DM Tool Suite

The AI dungeon master affects the game world through structured tool calls:

| Tool | Purpose |
|------|---------|
| `narrate()` | Display text to player |
| `spawn_npc()` | Create new character entity |
| `set_dc()` | Set difficulty for upcoming roll |
| `trigger_event()` | Fire a game event |
| `modify_rel()` | Change relationship edge weight |
| `update_var()` | Set/modify simulation variables |
| `grant_card()` | Give player a new card |
| `revoke_card()` | Remove a card (lost ally, etc.) |
| `set_mood()` | Change NPC emotional state |
| `check_law()` | RAG lookup against active laws |
| `roll_dice()` | Trigger a skill check |
| `branch_scene()` | Create narrative fork |
| `schedule_event()` | Queue future event N turns ahead |
| `end_scene()` | Close current interaction |
| `score_adjust()` | Modify player metrics |

## DM Operating Modes

### Narrator Mode
Between actions. Generates weekly briefings, describes consequences.
- Input: world state + recent player actions + relevant events
- Output: `narrate()`, `schedule_event()`, `update_var()`

### Conversation Mode
Player talking to NPCs (1-on-1 or group).
- Input: NPC profiles + relationship history + player text/audio
- Output: `narrate()` for dialogue, `modify_rel()`, `set_mood()`, `grant_card()`/`revoke_card()`, `roll_dice()`

### Dungeon Master Mode
Setting up and adjudicating events.
- Input: event type + involved parties + world conditions
- Output: `set_dc()`, `branch_scene()`, `trigger_event()`, `narrate()`

### Law Interpreter Mode
Checking whether actions comply with or are affected by active laws.
- Input: proposed action + active law database (RAG retrieval)
- Output: `check_law()` results, `narrate()` legal consequences, `roll_dice()` for ambiguity

## Context Management

### Context Budget (per inference call)

| Component | ~Tokens |
|-----------|---------|
| System prompt (tone + rules) | 500 |
| World summary (compressed sim state) | 1000 |
| Active scene | 500 |
| Relevant NPCs (max 5 × ~200) | 1000 |
| Conversation buffer (recent dialogue) | 1500 |
| Tool definitions | 500 |
| **Total** | **~5000** |

### Strategy

- Aggressive summarization + RAG retrieval
- Full NPC memories stored in RocksDB, retrieved on-demand when NPC enters scene
- World state compressed by a dedicated summarizer pass
- Conversation history sliding window with summary of older exchanges
- KV-cache reuse between calls via Candle

## Editable Prompt System

```
game/prompts/
├─ tone.toml             narrative style (gritty → satirical dial)
├─ dm_system.toml        DM rules, adjudication guidelines
├─ legal_style.toml      how to convert player law drafts to legal language
├─ npc_templates/        personality archetypes for NPC generation
└─ event_templates/      narrative templates for event types
```

All TOML — human-readable, moddable, version-controllable. Players can edit tone.toml to change the entire narrative personality.

## DM Behavioral Rules

### SHOULD
- Narrate consequences vividly
- Voice NPCs with personality
- Set appropriate DCs based on context
- Build custom event frameworks when player goes off-script
- Weave player actions into coherent ongoing narrative
- Foreshadow consequences ("Martinez looked uneasy...")
- Surprise the player with emergent situations

### SHOULD NOT
- Override system outcomes because they're "not dramatic enough"
- Fudge dice rolls
- Protect the player from consequences
- Railroad toward a "better story"
- Ignore simulation state for narrative convenience
- Make NPCs act against their personality/goals for plot
