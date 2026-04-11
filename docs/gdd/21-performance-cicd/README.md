---
title: Performance, Hardware & CI/CD
section: 21
status: design-complete
depends_on: [01]
blocks: []
---

# Performance, Hardware & CI/CD

## Hardware Requirements

### Minimum (Gemma 4B-it Q8)
- CPU: 4 cores, modern x86_64 or Apple Silicon
- RAM: 8 GB
- Storage: 5 GB (model ~3GB + game data + saves)
- GPU: none required (CPU inference viable for 4B)
- OS: macOS 12+, Linux (glibc 2.31+), Windows 10+
- Terminal: 80×24 min, 120×40 recommended, true color
- **Note**: 4B is "Lite Mode" — basic DM, tool calling less reliable

### Recommended (Gemma 12B-it Q4_K_M)
- CPU: 8 cores
- RAM: 16 GB (model ~8GB + game state + headroom)
- GPU: optional — 8GB+ VRAM for GPU-accelerated inference (Metal, CUDA, Vulkan)
- Storage: 15 GB
- Audio: microphone for voice input (optional, requires `audio` feature)
- **Sweet spot**: reliable tool calling, good narrative quality, 6-10s response with streaming

### Enthusiast (Gemma 27B-it Q4_K_M)
- RAM: 48 GB+ (or 24GB VRAM GPU)
- GPU: RTX 4090 (24GB), M3 Max (48GB+), A100
- Best narrative quality, most nuanced NPC conversations
- Quantized Q4 fits in 24GB VRAM; Q8 needs 48GB+

## Model Selection (First Launch)

Auto-detects hardware and recommends:

```
POLIT detected your hardware:
CPU: Apple M3 Pro │ RAM: 18GB │ GPU: integrated 18GB

Recommended: Gemma 12B-it Q4_K_M (~6-10s responses)

[1] Gemma 4B-it Q8    — fast, lite mode          (3.2 GB)
[2] Gemma 12B-it Q4   — balanced (recommended)   (7.8 GB)  ✓
[3] Gemma 27B-it Q4   — best quality, slower      (17 GB)
[4] Custom model path (GGUF)
```

Model downloaded from Hugging Face Hub on first run.

## Performance Benchmarks

### Generation Speed (tokens/second, 200-token response)

| Model | Apple M2 Pro | Apple M3 Max | RTX 4090 | 8-core CPU only |
|-------|-------------|-------------|----------|-----------------|
| 4B Q8 | ~45 tok/s | ~60 tok/s | ~100 tok/s | ~15-20 tok/s |
| **12B Q4** | **~20 tok/s** | **~35 tok/s** | **~50-65 tok/s** | **~5-8 tok/s** |
| 27B Q4 | OOM | ~18 tok/s | ~25-35 tok/s | ~2-4 tok/s |

### Response Times (200-token DM response, with streaming)

| Model | Time to First Token | Full Response | User Experience |
|-------|-------------------|---------------|-----------------|
| 4B Q8 (M3) | ~0.5s | ~3s | Instant feel |
| 12B Q4 (M3) | ~1s | ~6-10s | Good with typewriter |
| 27B Q4 (4090) | ~1s | ~6-8s | Good with typewriter |
| 12B Q4 (CPU) | ~3s | ~25-40s | Acceptable, needs streaming |

## Performance Architecture

### Bottleneck
LLM inference. Everything else is fast (simulation tick < 1ms).

### Strategy
Minimize inference calls, maximize their value.

### Inference Budget Per Turn

| Call | Tokens Out | When |
|------|-----------|------|
| Dawn briefing | ~500 | Every turn |
| Per conversation exchange | ~200 | Player in conversation |
| Per event narration | ~300 | Event triggers |
| Per custom action eval | ~400 | Freeform action |
| Dusk summary | ~200 | Every turn |
| **Typical turn total** | **3-8 calls** | — |

### Optimizations
- **Batch NPC actions**: Single inference call generates all NPC autonomous actions for the week
- **Pre-compute context**: While player reads/types, background thread compresses world state for next call
- **Cache evaluations**: Law compliance checks cached if law hasn't changed
- **Streaming output**: Token-by-token rendering via crossbeam channel → typewriter effect hides latency
- **KV-cache**: ort (ONNX Runtime) manages KV-cache automatically between calls
- **Simulation is pure Rust**: Economy, demographics, social graph — all sub-millisecond per tick

### Async Thread Architecture

| Thread | Responsibility | Framework |
|--------|---------------|-----------|
| UI | Ratatui render loop (60fps) | crossterm event loop |
| Game | ECS tick, simulation systems | bevy_ecs schedule |
| AI | ort (ONNX Runtime) inference (non-blocking) | tokio async |
| IO | RocksDB reads/writes | crossbeam channels |
| Audio | cpal mic/TTS (if enabled) | cpal callbacks |

Communication: crossbeam channels. **UI never freezes during AI inference.**

### RocksDB Performance
- Column family per domain (parallel reads across domains)
- Write-ahead log for crash safety
- LRU block cache ~256MB for game state reads
- Bloom filters on relationship and info entity lookups
- Compaction during downtime phase (player isn't waiting)
- Snapshots for saves (instant, copy-on-write)

## Cargo Dependencies

```toml
[dependencies]
# ECS Engine
bevy_ecs = "0.18"

# TUI
ratatui = "0.30"
crossterm = "0.28"
rat-widget = "2.5"                  # extended widgets for overlays

# AI Inference
ort = "0.4"                   # Gemma inference with tool calling

# Database
rust-rocksdb = "0.46"               # zaidoon1 fork, most maintained

# Graph
petgraph = "0.8"

# Scripting
rhai = "1.24"

# Threading
crossbeam = "0.8"
crossbeam-channel = "0.5"

# Serialization
serde = { version = "1", features = ["derive"] }
toml = "0.8"
serde_json = "1"

# HTTP (data pipeline)
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }

# Optional: Audio
cpal = { version = "0.16", optional = true }
whisper-rs = { version = "0.12", optional = true }
hound = { version = "3.5", optional = true }
rubato = { version = "0.15", optional = true }

[dev-dependencies]
criterion = "0.5"

[features]
default = []
audio = ["cpal", "whisper-rs", "hound", "rubato"]
gpu-cuda = ["ort/cuda"]
gpu-metal = ["ort/metal"]
gpu-vulkan = ["ort/vulkan"]
sim = []  # headless simulation test mode
```

## CI/CD Pipeline

### CI (GitHub Actions) — on push/PR

```
┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐
│  Lint   │  │  Test   │  │  Test   │  │  Test   │
│(clippy, │  │(unit)   │  │(integ)  │  │(scenario│
│ rustfmt)│  │         │  │         │  │ valid.) │
└────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘
     └──────┬─────┴─────┬──────┘            │
            ▼           ▼                    ▼
      ┌──────────┐ ┌──────────┐  ┌─────────────────┐
      │  Build   │ │ Headless │  │ SDK validation  │
      │ (release)│ │  sim run │  │ (example mods   │
      │ all OS   │ │ 50 turns │  │  build + test)  │
      └────┬─────┘ └────┬─────┘  └────────┬────────┘
           └──────┬─────┘                 │
                  ▼                       │
         ┌───────────────┐                │
         │  Balance      │◄───────────────┘
         │  check        │
         └───────────────┘
```

### Test Categories

**Unit Tests**: Economic model I/O, dice distribution, card interactions, social graph propagation, law enforcement classification, information spread, election math, Rhai sandbox security.

**Integration Tests**: Full turn cycle, law lifecycle, NPC lifecycle, card lifecycle, save/load roundtrip, mod loading, data pipeline.

**Headless Simulation Tests**: Mock AI (deterministic tool calls), run 100+ turns. Verify: economy in bounds, NPCs balanced, elections on schedule, laws enforce, info spreads correctly, no panics/infinite loops, memory bounded.

**Balance Tests** (weekly, expensive): Monte Carlo 1000 games. No dominant strategy, no unwinnable starts, progression feels natural. Generate balance report.

### Release Pipeline — on tag `v*`

**Cross-compile**: x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos, x86_64-windows

**Package**: Binary + game data + SDK tools + seed data

**Distribute**: GitHub Releases, crates.io (library), Homebrew tap, AUR, Flathub, winget

Model weights NOT bundled — downloaded at runtime from Hugging Face Hub.

### Development Commands

| Command | Purpose |
|---------|---------|
| `cargo run` | Launch (debug) |
| `cargo run -- --headless` | Headless simulation |
| `cargo run -- --mock-ai` | Deterministic AI for testing |
| `cargo run -- --tutorial` | Jump to tutorial |
| `cargo test` | Unit + integration |
| `cargo test --features sim` | Headless simulation tests |
| `cargo bench` | Performance benchmarks |
| `polit-data fetch --all` | Refresh real-world data |
| `polit-sdk validate game/` | Validate scenario data |
| `make venv` | Create Python virtual environment for simulation |
| `make venv-update` | Update Python dependencies in venv |
| `make sim-test` | Run Rust bridge tests + Python pytest suite |

## Python Simulation Stack

### Feature Flag

The Python simulation stack is behind the `simulation` Cargo feature flag:

```toml
[dependencies]
pyo3 = { version = "0.24", optional = true }
rmp-serde = "1"

[features]
simulation = ["pyo3"]
```

### macOS Setup

Homebrew Python requires the `PYO3_PYTHON` environment variable:

```bash
export PYO3_PYTHON=$(brew --prefix python@3.12)/bin/python3.12
```

### Performance Target

Python tick budget: **<500ms** for all 8 simulation layers (macro, sectors, markets, household, political, media, corporate, geopolitical). Current measured time is approximately 0.5s with all 156 tests passing.

### Test Suite

156 Python tests across 10 test files covering all simulation layers, the host entry point, and MessagePack serialization round-trips. Run via `make sim-test` which executes both `cargo test --features simulation` (Rust bridge tests) and `pytest sim/tests/` (Python unit tests).
