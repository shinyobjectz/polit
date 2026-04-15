# POLIT MCP Live Playtesting Design

**Date:** 2026-04-15  
**Issue:** `polit-r3yv`

## Goal

Add a stdio MCP server that lets Codex launch and play one real `polit` binary session turn by turn, using the same player-facing runtime that a human uses.

The MCP should make Codex effective at self-directed playtesting, feature validation, and regression checking without requiring a human operator to drive the terminal.

## Why This Exists

`poldev` now gives deterministic scenarios and PTY-backed runtime validation, but it is still shaped like a test harness. It is excellent for scripted checks and weak for open-ended interactive play.

For full-project testing, Codex needs a live control surface over a single real game session:

- launch the game
- press keys and type text
- inspect the current visible screen
- wait for transitions
- optionally capture screenshots
- read limited save/config/log context between turns

That moves validation from “run this scenario” to “actually play the game and investigate behavior.”

## Product Behavior

### Session Model

The MCP owns exactly one active `polit` session at a time.

That session is always backed by the real compiled `polit` binary running inside a pseudo-terminal. The server does not simulate state, reimplement screens, or bypass the runtime control path.

### Control Model

Gameplay control stays keyboard-driven so the MCP behaves like a player:

- send keys
- type text
- wait for visible text
- resize terminal
- inspect visible screen

The MCP may also expose supporting observability helpers such as save metadata and recent logs, but those are secondary to the terminal session itself.

### Observability Rules

The server is allowed to expose more than on-screen text, but responses must stay bounded and compact by default.

Principles:

- text summaries first
- screenshots only when needed
- file reads must be explicit and bounded
- save/log/config access should be limited to whitelisted areas
- no raw full-save dumps by default

The point is to make Codex effective without turning every MCP response into context spam.

## Architecture

Build a separate binary, likely `src/bin/polit_mcp.rs`, that runs as a stdio MCP server and owns one live `polit` child process at a time.

The implementation should have three layers:

### SessionManager

Responsible for:

- tracking whether a session exists
- launching and terminating the child process
- preventing duplicate concurrent sessions
- managing artifact paths such as screenshots
- tracking session metadata such as pid, start time, terminal size, and screen revision

### PtyRuntime

Responsible for:

- launching the real compiled `polit` binary in a PTY
- sending keypresses and typed text
- resizing the terminal
- continuously draining PTY output
- maintaining a parsed VT screen buffer
- capturing screenshots when requested

This should reuse the PTY lessons from `poldev`, but unlike `poldev`, it must support a long-lived interactive session instead of a finite scripted scenario.

### Inspector

Responsible for:

- summarizing the visible screen into compact text
- returning targeted visible line slices
- exposing bounded reads of save/config/log files
- keeping MCP responses small enough for agent iteration

This layer is where context discipline is enforced.

## Tool Surface

The MCP should expose a narrow set of composable tools.

### `launch`

Launch one real `polit` binary session.

Inputs:

- terminal width and height
- optional binary path override
- optional HOME override

Outputs:

- session status
- pid
- terminal size
- initial screen summary
- initial screen revision

### `send_keys`

Send keyboard input to the active session.

Inputs:

- key list and/or text
- optional settle timeout

Outputs:

- updated screen revision
- compact screen summary
- optional visible line slice

### `read_screen`

Read the current visible terminal screen in a bounded format.

Inputs:

- optional max lines
- optional line window

Outputs:

- screen revision
- compact summary
- visible text lines

### `wait_for_text`

Poll until text appears or timeout is reached.

Inputs:

- target text
- timeout

Outputs:

- found / not found
- final screen revision
- final summary

### `resize`

Resize the live PTY.

Inputs:

- width
- height

Outputs:

- updated size
- updated summary

### `screenshot`

Capture the terminal view when text alone is insufficient.

Inputs:

- optional label

Outputs:

- saved artifact path or handle

### `read_save_metadata`

Read compact campaign metadata only.

Outputs might include:

- save slots
- campaign name
- player role
- in-game date
- current location / mode if available

### `read_recent_logs`

Return bounded tail slices from known runtime logs.

Inputs:

- log kind
- max lines

### `read_file_excerpt`

Read bounded excerpts from whitelisted files only.

Inputs:

- whitelisted path
- byte or line range

### `terminate`

Cleanly stop the live session and release PTY resources.

Outputs:

- final status
- cleanup result

## Runtime Behavior

`launch` starts the compiled `polit` binary in a PTY and begins tracking live screen state immediately. PTY output is parsed continuously into a screen buffer and exposed through screen revisions so Codex can reason about whether anything changed.

`send_keys` is the main control primitive. After input, the runtime should settle briefly, drain output, update screen state, and return the smallest useful screen response. The MCP should avoid forcing a separate `read_screen` after every action unless explicitly needed.

`read_screen` should default to a compact summary plus visible lines rather than dumping the entire terminal indiscriminately. `screenshot` is a secondary tool for ambiguous layouts or when visual structure matters more than text.

`wait_for_text` replaces blind sleeps and should be the preferred synchronization primitive for menus, transitions, and turn results.

## Testing Strategy

The first milestone should validate the MCP against real startup and title interactions:

1. launch on a clean HOME reaches the AI setup gate
2. keyboard input through setup advances the runtime
3. `read_screen` and `wait_for_text` stay synchronized with PTY output
4. `terminate` reliably cleans up the child and PTY
5. `screenshot` works on demand

After that, add one true playtest smoke path:

1. launch
2. navigate setup and title
3. start a campaign
4. submit at least one real gameplay action
5. inspect resulting screen plus bounded save/log context
6. quit or terminate cleanly

## Safety Constraints

- only one active session at a time
- no arbitrary filesystem reads
- bounded outputs everywhere
- explicit errors for stale or crashed sessions
- no fake or inferred state transitions presented as fact

The MCP should only report what the real binary and allowed support files actually show.

## Recommendation

Build the MCP as a binary-session driver over the real compiled `polit` binary, not as a wrapper around scenario execution and not as a high-level semantic automation layer.

That keeps it honest, keeps it close to the player runtime, and gives Codex a stable foundation for real playtesting instead of synthetic validation.
