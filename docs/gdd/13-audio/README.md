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

## Voice Input

- Press `[v]` to record, `Enter` to send
- Works in any phase — free roam, conversation, speech, debate, anything with text input
- Gemma 4 E2B/E4B processes audio natively (no separate STT step)
- For speech/debate minigames: tone analysis adds roll modifiers
  - Strong delivery: +3 Persuasion
  - Nervous delivery: -2 Persuasion, +1 Sympathy
  - Angry delivery: +2 Intimidation, -1 Likability
- For regular chat input: just transcribes to text, no bonus
- **Never required** — typing always works identically

## Voice Output (TTS)

- Optional — reads DM narration aloud
- Single narrator voice (system TTS or Piper)
- No NPC voice differentiation
- Toggle with `ctrl+t`, off by default
- Purely convenience / accessibility

## Speech-Enhanced Gameplay

Certain moments benefit from voice:

| Moment | Voice Advantage |
|--------|----------------|
| Rally speech | Delivery analyzed for modifiers |
| Debate response | Real-time pressure, tone matters |
| Press conference | Quick responses feel authentic |
| Phone call | Whisper = conspiratorial, shout = intimidation |

## Configuration

```toml
# config/audio.toml

[input]
enabled = false
backend = "gemma4_native"  # or "whisper" for fallback
sample_rate = 16000
max_duration_secs = 120
silence_threshold = 0.01
silence_duration_ms = 2000
device = "default"

[output]
enabled = false
backend = "system"  # "system", "piper", "none"
speed = 1.0
volume = 0.8

[keybinds]
toggle_recording = "v"
stop_recording = "Enter"
toggle_tts = "ctrl+t"
mute_all = "ctrl+m"
```

## Rust Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `cpal` | Cross-platform audio I/O |
| `whisper-rs` | Whisper.cpp bindings (fallback STT) |
| `hound` | WAV file reading/writing |
| `rubato` | Sample rate conversion |
