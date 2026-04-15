# Poldev

`poldev` is the developer CLI for validating POLIT's terminal runtime with repeatable YAML scenarios.

## Commands

```bash
cargo run --bin poldev -- tui run tests/tui/scenarios/first_launch_ai_setup_codex.yaml
cargo run --bin poldev -- tui run --mode in_process tests/tui/scenarios/first_launch_ai_setup_codex.yaml
cargo run --bin poldev -- tui run --mode pty tests/tui/scenarios/first_launch_ai_setup_codex.yaml
```

- No `--mode`: use the scenario's declared mode. `mode: both` runs in-process first, then PTY.
- `--mode in_process`: run the deterministic in-process harness only.
- `--mode pty`: run the real compiled `polit` binary in a pseudo-terminal.

## Modes

- `in_process` links directly to the runtime, disables audio, injects keyboard input, and captures exact `ratatui` frames. Use it for fast iteration and deterministic assertions.
- `pty` drives the actual compiled `polit` binary through a pseudo-terminal and parses the visible terminal buffer. Use it to validate the true startup boundary, alternate screen behavior, and real terminal I/O.

## Scenario Files

Scenarios live in `tests/tui/scenarios/` and are written in YAML.

Core fields:

- `name`
- `mode`: `in_process`, `pty`, or `both`
- `terminal.width` and `terminal.height`
- `startup.command`
- `steps`
- `expect.running`

Supported step types:

- `press`
- `type`
- `assert_text`
- `assert_not_text`
- `snapshot`

Scenarios can also seed files into a temp HOME and assert on saved files, which is how startup and provider setup flows are validated today.

## Failure Output

When a run fails, `poldev` prints:

- scenario name
- backend mode
- failing step number and step description
- temp HOME path
- recent keyboard input history
- the latest visible frame dump

That output is designed to be directly actionable for agent-driven debugging.
