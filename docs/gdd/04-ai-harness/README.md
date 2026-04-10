---
title: AI Harness & Gemma Integration
section: 04
status: design-complete
depends_on: [01]
blocks: [10, 13]
---

# AI Harness & Gemma Integration

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   AI HARNESS                             │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Context Builder                      │   │
│  │  World State → compressed summary of current sim  │   │
│  │  Character Info → player stats, cards, rels       │   │
│  │  Scene Context → what's happening right now       │   │
│  │  NPC Profiles → relevant characters + memories    │   │
│  │  Tone Prompt → loaded from editable .toml file    │   │
│  │  History Window → recent events + conversation    │   │
│  └───────────────────────┬─────────────────────────┘   │
│                          │                              │
│         ┌────────────────┼────────────────┐             │
│         │ TEXT INPUT      │ AUDIO INPUT    │             │
│         │ (player types)  │ (player speaks)│             │
│         │                │      │         │             │
│         │                │  ┌───▼───────┐ │             │
│         │                │  │ whisper-rs │ │             │
│         │                │  │ (STT)     │ │             │
│         │                │  └───┬───────┘ │             │
│         │                │      │ text    │             │
│         └────────────────┼──────┘         │             │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐   │
│  │              ort (ONNX Runtime)                           │   │
│  │                                                   │   │
│  │  Model: Gemma 12B-it GGUF Q4_K_M (recommended)  │   │
│  │  Tool calling: OpenAI-compatible format           │   │
│  │  Constrained decoding: GBNF grammar               │   │
│  │  Output: guaranteed valid JSON tool calls         │   │
│  │  Streaming: token-by-token for typewriter effect  │   │
│  └───────────────────────┬─────────────────────────┘   │
│                          │                              │
│  ┌───────────────────────▼─────────────────────────┐   │
│  │              Tool Router                          │   │
│  │  Parses tool calls → ECS commands                │   │
│  │  JSON parsing trivial — ort (ONNX Runtime) guarantees    │   │
│  │  valid output via grammar-constrained decoding   │   │
│  └─────────────────────────────────────────────────┘   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Why ort (ONNX Runtime) (not raw Candle or llama.cpp)

| Feature | Candle | llama-cpp-2 | ort (ONNX Runtime) |
|---------|--------|-------------|------------|
| Gemma support | Flaky, open issues | Via GGUF (when supported) | Full native support |
| Tool calling | DIY | Via GBNF grammar (manual) | Built-in, OpenAI-compatible |
| Structured output | DIY | GBNF grammar | Grammar-constrained decoding |
| Streaming | Basic iterator | Yes | Full streaming API |
| Quantization | Manual | GGUF native | GGUF/GPTQ native |
| Embeddable | Tensor library (low-level) | C++ bindings | High-level Rust API |
| KV-cache | Basic | Managed | Managed automatically |

**ort (ONNX Runtime) eliminates the need to build**: tool call parser, JSON validator, KV-cache manager, constrained decoder. The `ai/harness.rs` module goes from ~2000 lines of custom inference code to ~200 lines of API calls.

## Inference Pipeline

### Audio Input Path
```
Microphone → cpal (capture) → PCM audio → whisper-rs (STT) → text
```
Audio is always converted to text before reaching the LLM. Speech analysis for gameplay bonuses (delivery, tone, pacing) is performed by the LLM via text description of audio characteristics, not raw audio processing.

### Text Processing Path
```
Context Builder → system prompt + user text → ort (ONNX Runtime) → tool call JSON → Tool Router → ECS commands
```

### Streaming Output
```
ort (ONNX Runtime) generates tokens → streamed to UI thread via crossbeam channel → 
Ratatui renders typewriter effect → hides inference latency
```

## DM Tool Suite

The AI dungeon master affects the game world through structured tool calls. ort (ONNX Runtime) uses OpenAI-compatible tool calling format with GBNF grammar constraints to guarantee valid JSON.

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

### GBNF Grammar for Tool Calls

ort (ONNX Runtime) supports GBNF (GGML BNF) grammar to constrain output. We define a grammar that only allows valid tool call JSON matching our schema. This eliminates malformed tool calls entirely — no retry logic needed.

```
root   ::= "{" ws "\"tool\"" ws ":" ws tool-name ws "," ws "\"args\"" ws ":" ws "{" ws args ws "}" ws "}"
tool-name ::= "\"narrate\"" | "\"spawn_npc\"" | "\"set_dc\"" | ...
```

## DM Operating Modes

### Narrator Mode
Between actions. Generates weekly briefings, describes consequences.
- Input: world state + recent player actions + relevant events
- Output: `narrate()`, `schedule_event()`, `update_var()`

### Conversation Mode
Player talking to NPCs (1-on-1 or group).
- Input: NPC profiles + relationship history + player text
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

With 128K context available on Gemma 12B+, we can afford much richer context than originally planned:

| Component | Tokens | Notes |
|-----------|--------|-------|
| System prompt (tone + rules) | 1,000 | Includes tool definitions |
| World summary (compressed sim state) | 2,000 | Economy, politics, active crises |
| Active scene | 1,000 | Current situation detail |
| Relevant NPCs (max 5 × ~400) | 2,000 | Personality, memories, relationship |
| Conversation buffer | 3,000 | Recent dialogue with summaries of older |
| Active laws (RAG-retrieved, if relevant) | 1,000 | Only when law interpretation needed |
| **Total budget** | **~10,000** | Well within 128K, leaves headroom |

### Strategy

- Aggressive summarization + RAG retrieval from RocksDB
- Full NPC memories stored in RocksDB, retrieved on-demand when NPC enters scene
- World state compressed by a dedicated summarizer pass (can itself be an LLM call during downtime)
- Conversation history sliding window with summary of older exchanges
- KV-cache managed automatically by ort (ONNX Runtime) between calls

## Model Selection

| Tier | Model | GGUF Quant | VRAM/RAM | Speed (Apple Silicon) | Speed (RTX 4090) |
|------|-------|-----------|----------|----------------------|-------------------|
| Budget | Gemma 4B-it | Q8_0 | ~5 GB | ~45 tok/s | ~100 tok/s |
| **Recommended** | **Gemma 12B-it** | **Q4_K_M** | **~8 GB** | **~20-35 tok/s** | **~50-65 tok/s** |
| Enthusiast | Gemma 27B-it | Q4_K_M | ~18 GB | ~18 tok/s | ~25-35 tok/s |

**12B Q4 is the sweet spot.** At 20-35 tok/s on Apple Silicon, a 200-token DM response takes 6-10 seconds — acceptable with streaming typewriter effect. Tool calling is reliable at 12B+. The 4B model works for gameplay but tool calling becomes unreliable — labeled "Lite Mode" in settings.

## Editable Prompt System

```
game/prompts/
├─ tone.toml             narrative style (gritty → satirical dial)
├─ dm_system.toml        DM rules, adjudication guidelines
├─ legal_style.toml      how to convert player law drafts to legal language
├─ npc_templates/        personality archetypes for NPC generation
└─ event_templates/      narrative templates for event types
```

All TOML — human-readable, moddable, version-controllable.

## DM Behavioral Rules

### SHOULD
- Narrate consequences vividly
- Voice NPCs with personality
- Set appropriate DCs based on context
- Build custom event frameworks when player goes off-script
- Weave player actions into coherent ongoing narrative
- Foreshadow consequences
- Surprise the player with emergent situations

### SHOULD NOT
- Override system outcomes because they're "not dramatic enough"
- Fudge dice rolls
- Protect the player from consequences
- Railroad toward a "better story"
- Ignore simulation state for narrative convenience
- Make NPCs act against their personality/goals for plot
