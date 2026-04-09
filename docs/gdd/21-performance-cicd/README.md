---
title: Performance, Hardware & CI/CD
section: 21
status: design-complete
depends_on: [01]
blocks: []
---

# Performance, Hardware & CI/CD

## Hardware Requirements

### Minimum (Gemma 4 E2B — 2B params)
- CPU: 4 cores, modern x86_64 or Apple Silicon
- RAM: 8 GB
- Storage: 5 GB
- GPU: none required
- OS: macOS 12+, Linux (glibc 2.31+), Windows 10+
- Terminal: 80×24 min, 120×40 recommended, true color

### Recommended (Gemma 4 E4B — 4B params)
- CPU: 8 cores
- RAM: 16 GB
- GPU: optional, 6GB VRAM for acceleration
- Storage: 10 GB
- Audio: microphone (optional)

### Enthusiast (Gemma 4 27B — full model)
- RAM: 32 GB
- GPU: 16-24GB VRAM (RTX 4090, A100, M2 Ultra)
- Quantized models (Q4/Q8) for lower VRAM

## Model Selection (First Launch)

Auto-detects hardware and recommends:

```
POLIT detected your hardware:
CPU: Apple M3 Pro │ RAM: 18GB │ GPU: integrated 18GB

Recommended: Gemma 4 E4B (~2-4s response time)

[1] Gemma 4 E2B  — fast, good quality        (1.5 GB)
[2] Gemma 4 E4B  — balanced (recommended)    (3.2 GB)  ✓
[3] Gemma 4 27B  — best quality, slower      (16 GB)
[4] Custom model path (GGUF/ONNX)
```

Model downloaded from Hugging Face Hub on first run.

## Performance Architecture

### Bottleneck
LLM inference. Everything else is fast.

### Strategy
Minimize inference calls, maximize their value.

### Inference Budget Per Turn
| Call | Tokens Out |
|------|-----------|
| Dawn briefing | ~500 |
| Per conversation exchange | ~200 |
| Per event narration | ~300 |
| Per custom action eval | ~400 |
| Dusk summary | ~200 |
| **Typical turn total** | **3-8 calls** |

### Optimizations
- Batch NPC autonomous actions into single inference call
- Pre-compute context summaries during player think time
- Cache common evaluations (law compliance if law unchanged)
- Streaming output: typewriter effect hides latency
- KV-cache reuse between calls via Candle
- Simulation systems are pure Rust — sub-millisecond per tick

### Async Architecture

| Thread | Responsibility | Framework |
|--------|---------------|-----------|
| UI | Ratatui render loop (60fps) | crossterm event loop |
| Game | ECS tick, simulation systems | bevy_ecs schedule |
| AI | Candle inference (non-blocking) | tokio async |
| IO | RocksDB reads/writes | crossbeam channels |
| Audio | CPAL mic/TTS (if enabled) | cpal callbacks |

Communication: crossbeam channels. UI never freezes during AI inference.

### RocksDB Performance
- Column family per domain (parallel reads)
- Write-ahead log for crash safety
- LRU block cache ~256MB
- Bloom filters on relationship/info lookups
- Compaction during downtime phase
- Snapshots for saves (instant, copy-on-write)

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
      │ (release)│ │  sim run │  │ (example mods)  │
      │ all OS   │ │ 50 turns │  └────────┬────────┘
      └────┬─────┘ └────┬─────┘           │
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

**Headless Simulation Tests**: Mock AI, run 100+ turns. Verify: economy in bounds, NPCs balanced, elections on schedule, laws enforce, info spreads correctly, no panics/infinite loops, memory bounded.

**Balance Tests** (weekly, expensive): Monte Carlo 1000 games. No dominant strategy, no unwinnable starts, progression feels natural. Generate balance report.

### Release Pipeline — on tag `v*`

**Cross-compile**: x86_64-linux, aarch64-linux, x86_64-macos, aarch64-macos, x86_64-windows

**Package**: Binary + game data + SDK tools + seed data

**Distribute**: GitHub Releases, crates.io (library), Homebrew tap, AUR, Flathub, winget

Model weights NOT bundled — downloaded at runtime.

### Development Workflow

| Command | Purpose |
|---------|---------|
| `cargo run` | Launch (debug) |
| `cargo run -- --headless` | Headless simulation |
| `cargo run -- --mock-ai` | Deterministic AI (testing) |
| `cargo test` | Unit + integration |
| `cargo test --features sim` | Headless simulation tests |
| `cargo bench` | Performance benchmarks |
| `polit-data fetch --all` | Refresh real-world data |
| `polit-sdk validate game/` | Validate scenario data |
| `cargo run -- --tutorial` | Jump to tutorial |

### Cargo Feature Flags

| Feature | Description |
|---------|-------------|
| `default = ["e2b"]` | Minimal, works everywhere |
| `e4b` | Gemma 4 E4B support |
| `full` | Gemma 4 27B support |
| `audio` | Voice input/output (cpal) |
| `gpu-cuda` | CUDA acceleration |
| `gpu-metal` | Metal acceleration (macOS) |
| `gpu-vulkan` | Vulkan acceleration |
| `sim` | Headless simulation test mode |
