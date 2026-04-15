# Poldev TUI Validation Harness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a `poldev` developer CLI that validates POLIT's `ratatui` runtime through shared YAML scenarios executed in both in-process and PTY modes.

**Architecture:** Add a new `poldev` binary with a shared scenario parser and two runners. First extract thin runtime seams around input, timing, terminal capture, and audio disabling so in-process validation can run current production code deterministically. Then add a PTY runner that executes the real `polit` binary against the same scenarios for true end-to-end validation.

**Tech Stack:** Rust, `ratatui`, `crossterm`, `serde_yaml`, `tempfile`, likely `clap`, plus PTY/terminal parsing crates such as `portable-pty` and `vt100`

---

### Task 1: Scaffold the poldev binary and scenario parser

**Files:**
- Create: `src/bin/poldev.rs`
- Create: `src/devtools/mod.rs`
- Create: `src/devtools/scenario.rs`
- Create: `tests/poldev_scenario.rs`
- Modify: `Cargo.toml`

**Step 1: Write the failing parser tests**

Add tests that load YAML into a `Scenario` type and verify:
- valid scenario parses
- unknown mode fails
- unknown step shape fails

Example target:

```rust
#[test]
fn parses_minimal_tui_scenario() {
    let yaml = r#"
name: smoke
mode: in_process
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - assert_text: "POLIT"
expect:
  running: true
"#;

    let scenario = Scenario::from_yaml(yaml).unwrap();
    assert_eq!(scenario.name, "smoke");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_scenario -- --nocapture`  
Expected: FAIL because `Scenario` parser does not exist yet

**Step 3: Add minimal parser implementation**

Implement:
- `Scenario`
- `ScenarioMode`
- `ScenarioStep`
- `Scenario::from_yaml(...)`

Prefer explicit enums over loose `serde_json::Value`.

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_scenario -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml src/bin/poldev.rs src/devtools/mod.rs src/devtools/scenario.rs tests/poldev_scenario.rs
git commit -m "feat(poldev): add scenario parser scaffold"
```

### Task 2: Add text frame dumping for deterministic assertions

**Files:**
- Create: `src/devtools/frame_dump.rs`
- Create: `tests/poldev_frame_dump.rs`
- Modify: `src/devtools/mod.rs`

**Step 1: Write the failing frame dump test**

Write a test that renders simple text into a `ratatui` test backend and expects a normalized text dump.

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_frame_dump -- --nocapture`  
Expected: FAIL because frame dump helpers do not exist

**Step 3: Implement minimal frame dump support**

Add helpers that convert a `ratatui` buffer into normalized lines with trailing whitespace trimmed consistently.

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_frame_dump -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/devtools/frame_dump.rs tests/poldev_frame_dump.rs src/devtools/mod.rs
git commit -m "feat(poldev): add frame dump utilities"
```

### Task 3: Extract startup harness seams from the UI runtime

**Files:**
- Create: `src/devtools/harness.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/title.rs`
- Modify: `src/ui/setup.rs`
- Modify: `src/ui/scenario.rs`
- Modify: `src/ui/character_creation.rs`
- Test: `tests/poldev_in_process_startup.rs`

**Step 1: Write the failing in-process startup test**

Write a test that drives startup in-process with scripted input and expects the first-launch AI setup screen when config is absent.

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_in_process_startup -- --nocapture`  
Expected: FAIL because runtime still hardcodes direct event polling or lacks harness injection

**Step 3: Implement minimal harness seams**

Add abstractions for:
- event source
- frame sink
- optional no-op audio
- deterministic tick/clock behavior where needed

Do not reimplement screens. Refactor them to accept the harness interfaces while preserving production behavior.

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_in_process_startup -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/devtools/harness.rs src/ui/mod.rs src/ui/title.rs src/ui/setup.rs src/ui/scenario.rs src/ui/character_creation.rs tests/poldev_in_process_startup.rs
git commit -m "refactor(ui): add harness seams for poldev"
```

### Task 4: Add the in-process runner

**Files:**
- Create: `src/devtools/in_process.rs`
- Modify: `src/devtools/mod.rs`
- Modify: `src/bin/poldev.rs`
- Test: `tests/poldev_in_process_runner.rs`

**Step 1: Write the failing runner test**

Write a test that:
- loads a YAML scenario
- runs it in-process
- asserts a visible string from the setup/title flow

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_in_process_runner -- --nocapture`  
Expected: FAIL because `InProcessRunner` is not implemented

**Step 3: Implement the minimal runner**

Support:
- terminal size setup
- keyboard `press` and `type`
- `assert_text`
- `assert_not_text`
- `snapshot`
- temp HOME directory wiring

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_in_process_runner -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/devtools/in_process.rs src/devtools/mod.rs src/bin/poldev.rs tests/poldev_in_process_runner.rs
git commit -m "feat(poldev): add in-process tui runner"
```

### Task 5: Add the first checked-in startup scenarios

**Files:**
- Create: `tests/tui/scenarios/first_launch_ai_setup_codex.yaml`
- Create: `tests/tui/scenarios/first_launch_ai_setup_openrouter_missing_key.yaml`
- Create: `tests/tui/scenarios/title_reopen_ai_setup.yaml`
- Test: `tests/poldev_scenarios.rs`

**Step 1: Write the failing scenario execution test**

Write a test that runs one checked-in scenario through the in-process runner and asserts success.

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_scenarios -- --nocapture`  
Expected: FAIL because the scenario files or runner support are incomplete

**Step 3: Add minimal scenarios**

Each scenario should use:
- temp HOME
- fixed terminal size
- keyboard-only steps
- at least one file assertion on `config/ai.toml` where relevant

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_scenarios -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add tests/tui/scenarios tests/poldev_scenarios.rs
git commit -m "test(poldev): add startup tui scenarios"
```

### Task 6: Add the PTY runner

**Files:**
- Create: `src/devtools/pty.rs`
- Modify: `Cargo.toml`
- Modify: `src/devtools/mod.rs`
- Modify: `src/bin/poldev.rs`
- Test: `tests/poldev_pty_runner.rs`

**Step 1: Write the failing PTY smoke test**

Write a test that launches the real `polit` binary in a PTY and asserts that startup text is captured from the terminal buffer.

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_pty_runner -- --nocapture`  
Expected: FAIL because PTY execution is not implemented

**Step 3: Implement minimal PTY execution**

Add:
- process launch for the compiled `polit` binary
- key injection
- terminal buffer parsing
- visible frame extraction
- timeout handling

Prefer a crate like `portable-pty` plus `vt100` rather than shelling out to `script`.

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_pty_runner -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml src/devtools/pty.rs src/devtools/mod.rs src/bin/poldev.rs tests/poldev_pty_runner.rs
git commit -m "feat(poldev): add pty tui runner"
```

### Task 7: Run a shared scenario in both modes

**Files:**
- Modify: `tests/poldev_scenarios.rs`
- Modify: `tests/tui/scenarios/first_launch_ai_setup_codex.yaml`

**Step 1: Write the failing dual-mode test**

Add a test that runs the same scenario once in-process and once in PTY mode.

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_scenarios -- --nocapture`  
Expected: FAIL because dual-mode scenario execution is not complete yet

**Step 3: Implement the minimal shared-mode support**

Make sure:
- mode override works from CLI/test code
- failure output identifies backend mode
- the same scenario data structure can drive both runners

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_scenarios -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add tests/poldev_scenarios.rs tests/tui/scenarios/first_launch_ai_setup_codex.yaml
git commit -m "test(poldev): run shared scenario across both backends"
```

### Task 8: Improve CLI diagnostics and snapshot output

**Files:**
- Modify: `src/bin/poldev.rs`
- Modify: `src/devtools/in_process.rs`
- Modify: `src/devtools/pty.rs`
- Create: `tests/poldev_cli.rs`

**Step 1: Write the failing CLI diagnostics test**

Write a test that runs a known-failing scenario and asserts the CLI reports:
- failing step index
- scenario name
- backend mode
- frame dump

**Step 2: Run test to verify it fails**

Run: `cargo test poldev_cli -- --nocapture`  
Expected: FAIL because diagnostics are not rich enough yet

**Step 3: Implement minimal diagnostics**

Add:
- structured failure formatting
- frame dump output
- recent input history
- temp-home path display

**Step 4: Run tests to verify they pass**

Run: `cargo test poldev_cli -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/poldev.rs src/devtools/in_process.rs src/devtools/pty.rs tests/poldev_cli.rs
git commit -m "feat(poldev): improve cli diagnostics"
```

### Task 9: Document developer usage

**Files:**
- Modify: `README.md`
- Create: `docs/poldev.md`

**Step 1: Write the failing doc checklist**

Create a short checklist in the task notes verifying docs include:
- how to run a scenario
- difference between `in-process` and `pty`
- where scenarios live
- how failures are reported

**Step 2: Verify docs are currently missing**

Run: `rg -n "poldev|tui run|in-process|pty" README.md docs || true`  
Expected: missing or incomplete references

**Step 3: Write minimal docs**

Add examples such as:

```bash
cargo run --bin poldev -- tui run tests/tui/scenarios/first_launch_ai_setup_codex.yaml
cargo run --bin poldev -- tui run --mode pty tests/tui/scenarios/first_launch_ai_setup_codex.yaml
```

**Step 4: Verify docs contain the new usage**

Run: `rg -n "poldev|tui run|in-process|pty" README.md docs/poldev.md`  
Expected: matching lines found

**Step 5: Commit**

```bash
git add README.md docs/poldev.md
git commit -m "docs(poldev): add tui harness usage"
```

### Task 10: Final verification

**Files:**
- Verify only

**Step 1: Run focused parser and harness tests**

Run:

```bash
cargo test poldev_scenario -- --nocapture
cargo test poldev_frame_dump -- --nocapture
cargo test poldev_in_process_runner -- --nocapture
cargo test poldev_pty_runner -- --nocapture
cargo test poldev_scenarios -- --nocapture
cargo test poldev_cli -- --nocapture
```

Expected: all pass

**Step 2: Run one scenario manually through both backends**

Run:

```bash
cargo run --bin poldev -- tui run tests/tui/scenarios/first_launch_ai_setup_codex.yaml
cargo run --bin poldev -- tui run --mode pty tests/tui/scenarios/first_launch_ai_setup_codex.yaml
```

Expected: both succeed

**Step 3: Verify git scope**

Run:

```bash
git status --short
git log --oneline -n 10
```

Expected: only intended `poldev` changes remain

**Step 4: Commit any final cleanup**

```bash
git add -A
git commit -m "chore(poldev): finalize tui harness rollout"
```

If no cleanup is needed, skip this commit.
