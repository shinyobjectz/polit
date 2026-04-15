# POLIT MCP

`polit_mcp` is a stdio MCP server for live playtesting the real `polit` terminal runtime one session at a time.

It is built for agent-driven testing:

- launches the compiled `polit` binary in a PTY
- sends keyboard input turn by turn
- reads the visible screen buffer without OCR
- exposes bounded save/config/log inspection helpers

## Run

Build the binaries first:

```bash
cargo build --bin polit --bin polit_mcp
```

Then run the MCP server on stdio:

```bash
target/debug/polit_mcp
```

## Session Model

- single live `polit` session at a time
- `launch` replaces any existing live session
- `terminate` kills the active session
- responses return bounded screen text plus a `screenRevision`

## Core Methods

### `launch`

Starts the real `polit` binary inside a pseudo-terminal.

Important params:

- `binaryPath`
- `home`
- `args`
- `pathEnv`
- `terminal.width`
- `terminal.height`

### `send_keys`

Types text and/or sends keys to the active session.

- `text` is typed first
- `keys` are sent after text, in order
- use `settleMs` to wait for the screen to update

### `read_screen`

Returns a bounded view of the visible terminal screen.

### `wait_for_text`

Waits until visible screen text appears or the timeout expires.

### `resize`

Resizes the PTY and returns the updated screen.

### `screenshot`

Writes a text artifact of the visible terminal screen under:

```text
~/.polit/mcp-artifacts/
```

### Inspect Helpers

- `read_save_metadata`
- `read_recent_logs`
- `read_file_excerpt`

These are intentionally bounded and only allow whitelisted paths under `~/.polit/`.

## Example Flow

Minimal request sequence:

1. `launch`
2. `read_screen` or `wait_for_text`
3. `send_keys`
4. `read_save_metadata` or `read_file_excerpt` when needed
5. `terminate`

## Observability Rules

- prefer `wait_for_text` and bounded `read_screen` for normal play
- use `screenshot` only when layout ambiguity matters
- use inspection helpers instead of arbitrary filesystem reads
- keep screen reads small to avoid context bloat during agent playtesting
