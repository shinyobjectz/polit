---
title: Audio System
section: 13
status: design-complete
depends_on: [04]
blocks: []
---

# Audio System

## Design Philosophy

Keep audio simple. Voice is an alternative input method and optional output for the player's own words. No NPC voices, no complex audio routing.

## Architecture

```
INPUT:   Microphone → cpal (capture) → PCM audio → whisper-rs (STT) → text → LLM
OUTPUT:  LLM narration text → system TTS or Piper (optional) → cpal (playback)
```

Audio input is always a **two-stage pipeline**: speech-to-text via whisper-rs, then text to the LLM. There is no native audio processing in the LLM — all audio is converted to text first.

## Voice Input

- Press `[v]` to record, `Enter` to send
- Works in any phase — free roam, conversation, speech, debate, anything with text input
- **whisper-rs** (Whisper.cpp Rust bindings) performs speech-to-text locally
- Transcribed text sent to ort (ONNX Runtime) as normal text input
- **Never required** — typing always works identically

### Speech Gameplay Bonuses

For speech/debate minigames, the system provides delivery bonuses through a simple analysis layer:

- **Duration**: Short responses vs. long speeches — affects different mechanics differently
- **Keyword density**: whisper-rs transcription analyzed for policy terms, emotional language, attacks
- **The LLM judges quality**: ort (ONNX Runtime) receives the transcribed text with a prompt to evaluate delivery and content, then sets roll modifiers accordingly

This is NOT raw audio tone analysis. The bonuses come from the LLM's assessment of the transcribed content:

| LLM Assessment | Modifier |
|---------------|----------|
| Strong, substantive speech | +3 Persuasion |
| Emotional, personal appeal | +2 Charisma, -1 Policy Knowledge |
| Aggressive, attacking | +2 Intimidation, -1 Likability |
| Vague, rambling | -2 Persuasion |
| Brief and punchy | +1 Media Savvy |

## Voice Output (TTS)

- Optional — reads DM narration aloud
- Single narrator voice (system TTS or Piper via ort if desired)
- No NPC voice differentiation
- Toggle with `ctrl+t`, off by default
- Purely convenience / accessibility

## Configuration

```toml
# config/audio.toml

[input]
enabled = false
model = "whisper-base.en"    # whisper model size: tiny, base, small, medium
sample_rate = 16000
max_duration_secs = 120
silence_threshold = 0.01
silence_duration_ms = 2000
device = "default"

[output]
enabled = false
backend = "system"           # "system", "piper", "none"
speed = 1.0
volume = 0.8

[keybinds]
toggle_recording = "v"
stop_recording = "Enter"
toggle_tts = "ctrl+t"
mute_all = "ctrl+m"
```

## Whisper Model Selection

| Model | Size | Speed (M3 Pro) | Accuracy | Recommended For |
|-------|------|----------------|----------|-----------------|
| tiny.en | 75 MB | Real-time | Good | Low-end hardware |
| base.en | 142 MB | Real-time | Better | **Default** |
| small.en | 466 MB | ~2x real-time | Great | Most users |
| medium.en | 1.5 GB | ~5x real-time | Excellent | Enthusiast |

All models run locally via whisper-rs (whisper.cpp Rust bindings). English-only (`.en`) variants are faster and more accurate for an American politics game.

## Rust Crate Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `cpal` | 0.16 | Cross-platform audio I/O (mic input, speaker output) |
| `whisper-rs` | 0.12 | Whisper.cpp bindings (local STT) |
| `hound` | 3.5 | WAV file reading/writing |
| `rubato` | 0.15 | Sample rate conversion |

All audio dependencies are behind the `audio` Cargo feature flag — not compiled unless the player enables voice input.
