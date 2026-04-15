# Poldev TUI Validation Harness Design

**Date:** 2026-04-15  
**Issue:** `polit-u6iu`

## Goal

Add a developer CLI named `poldev` that validates POLIT's `ratatui` runtime through two complementary modes:

- `in-process` for deterministic screen/state validation against the current runtime code
- `pty` for end-to-end validation against the real compiled `polit` binary

The harness should use a shared YAML scenario format so agents can author, patch, and re-run keyboard-driven flows without inventing one-off test code.

## Why This Exists

POLIT now has meaningful runtime UX that cannot be validated confidently with unit tests alone:

- first-launch AI setup gating
- title-screen navigation
- multi-screen startup transitions
- terminal-size-dependent rendering
- config-dependent behavior

Right now the only reliable validation path is manual playthrough. That is slow, hard to repeat, and poor for agent-driven iteration. `poldev` should become the stable path for validating TUI behavior from the terminal.

## Product Behavior

### Command Surface

`poldev` should be a separate developer-facing binary, likely at `src/bin/poldev.rs`.

Initial command surface:

- `poldev tui run <scenario.yaml>`
- `poldev tui run --mode in-process <scenario.yaml>`
- `poldev tui run --mode pty <scenario.yaml>`
- `poldev tui list-scenarios`
- `poldev tui snapshot <target>`

Later additions can include recording helpers, but the first version should prioritize reliable execution and diagnostics.

### Scenario Authoring

Scenarios should be checked into the repo as YAML files under a dedicated directory such as:

- `tests/tui/scenarios/`

Each scenario should declare:

- scenario name
- preferred mode: `in_process`, `pty`, or `both`
- terminal width and height
- startup target
- environment overrides
- temp-home setup instructions
- scripted keyboard steps
- assertions on visible text, file outputs, and final state

The format must be easy for an agent to write from scratch and easy to diff in code review.

### Modes

#### In-process mode

This mode should call real POLIT runtime code directly in the same Rust process. It is the fast inner-loop path.

Requirements:

- use deterministic terminal/frame capture
- inject keyboard input programmatically
- disable or stub audio
- allow fixed terminal sizing
- support temp HOME/config/save directories
- produce direct frame dumps on failure

This mode is for rapid debugging and deterministic assertions.

#### PTY mode

This mode should launch the actual `polit` binary inside a pseudo-terminal and drive it with real keypresses.

Requirements:

- use the compiled runtime boundary
- validate alternate-screen/raw-mode startup behavior
- capture terminal output through a VT parser, not OCR
- reuse the same YAML scenarios where possible

This mode is slower, but it is the truth path for real runtime validation.

## Architecture

### Shared scenario engine

`poldev` should parse a scenario once into a shared internal representation. The same scenario object should then be executable by either backend:

- `InProcessRunner`
- `PtyRunner`

This prevents drift between “fast” and “real” testing.

### Harness seams in POLIT

POLIT's current screens mostly own their own event loop by calling `crossterm::event::poll/read()` directly. That is fine for production, but it blocks in-process automation.

To support `poldev`, the runtime should gain a thin harness seam around:

- event source
- clock/tick source
- audio control
- terminal frame sink
- startup entrypoints

The important constraint is that `poldev` must drive current runtime-production code, not a separate test-only reimplementation.

### Frame capture

In-process mode should use `ratatui` `TestBackend` or an equivalent buffer capture abstraction. Every rendered frame should be convertible into plain text for:

- snapshot output
- assertion matching
- failure diagnostics

PTY mode should use a pseudo-terminal plus a VT parser to reconstruct the visible terminal state. It should not rely on screenshots or OCR.

### Audio

Audio should be disabled in both validation modes by default. The harness needs determinism, not audible behavior.

This likely means introducing a runtime audio abstraction or a no-op path that production uses only when the harness requests it.

## YAML Scenario Format

The first version should support keyboard-first flows. That covers the bulk of current POLIT runtime interactions.

Representative structure:

```yaml
name: first_launch_codex_setup
mode: both
terminal:
  width: 120
  height: 40
startup:
  command: app
env:
  HOME: temp
steps:
  - assert_text: "AI Setup"
  - press: Enter
  - assert_text: "Codex Subscription"
  - assert_text: "Validate Codex and save"
  - snapshot: after-codex-save
expect:
  running: true
files:
  - path: config/ai.toml
    contains: 'provider = "codex"'
```

Core step types:

- `press`
- `type`
- `wait`
- `resize`
- `assert_text`
- `assert_not_text`
- `assert_file`
- `snapshot`

This is enough to validate the current startup and setup flows.

## Failure Diagnostics

`poldev` should be opinionated about failure output. Every failed run should emit:

- failing step number and scenario name
- backend mode
- latest visible frame dump
- recent input history
- temp-home path
- relevant file assertion output

The failure output must be readable enough that an agent can patch code directly from the terminal log.

## Initial Coverage

The first milestone should validate:

1. first-launch AI setup appears before title when config is missing
2. title menu can reopen AI setup
3. an OpenRouter or Codex setup flow persists the expected config
4. the configured provider is then used in startup/runtime paths

Only after that should the harness expand into broader character-creation and gameplay flows.

## Testing Strategy

Testing should happen at three layers:

### Scenario parser tests

- YAML parsing
- validation of unknown step types
- validation of missing required fields

### In-process harness tests

- frame capture correctness
- keyboard step execution
- assertion failure formatting
- temp HOME setup behavior

### PTY harness tests

- binary launch works
- key injection works
- visible buffer extraction works
- at least one repo-kept scenario runs successfully against the real binary

## Out of Scope

The first version should not attempt:

- mouse automation
- OCR or screenshot-based validation
- generalized replay recording
- broad gameplay coverage beyond startup/setup/title validation
- replacing Rust tests with YAML entirely

## Recommendation

Build `poldev` with both modes from the start, but land the in-process path first because it will force the right harness seams and give immediate DX wins. Then add PTY execution against the real `polit` binary using the same YAML scenarios so the harness graduates from fast validation to true runtime validation without changing authoring workflows.
