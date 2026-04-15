# POLIT MCP Live Playtesting Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a stdio MCP server that launches one real compiled `polit` session in a PTY and lets Codex play the game turn by turn through keyboard-driven tools plus bounded observability helpers.

**Architecture:** Add a new `polit_mcp` binary with a single-session manager, a long-lived PTY runtime, and compact inspection helpers. Reuse the proven PTY mechanics from `poldev`, but reshape them around interactive session state and MCP tool handlers instead of finite YAML scenarios.

**Tech Stack:** Rust, stdio MCP server protocol, `portable-pty`, `vt100`, existing `poldev` PTY logic, likely `serde_json`, `tokio` or synchronous stdio handling, and existing runtime/save/log file paths.

---

### Task 1: Scaffold the MCP binary and session state

**Files:**
- Create: `src/bin/polit_mcp.rs`
- Create: `src/mcp/mod.rs`
- Create: `src/mcp/session.rs`
- Modify: `src/lib.rs`
- Test: `tests/polit_mcp_cli.rs`

**Step 1: Write the failing CLI/session test**

Write a test that starts the MCP binary in a minimal stdio mode, sends a `launch`-style request, and verifies that the binary recognizes the method shape even if session handling is not implemented yet.

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_cli -- --nocapture`  
Expected: FAIL because the MCP binary and session module do not exist yet

**Step 3: Add minimal MCP binary and session scaffold**

Implement:
- `polit_mcp` binary entrypoint
- basic stdio request loop
- `SessionManager` shell with one optional active session
- placeholder method dispatch for the planned tool names

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_cli -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/polit_mcp.rs src/mcp/mod.rs src/mcp/session.rs src/lib.rs tests/polit_mcp_cli.rs
git commit -m "feat(mcp): scaffold live playtesting server"
```

### Task 2: Extract reusable interactive PTY session runtime

**Files:**
- Create: `src/mcp/pty_session.rs`
- Modify: `src/devtools/pty.rs`
- Test: `tests/polit_mcp_pty_session.rs`

**Step 1: Write the failing PTY session test**

Write a test that launches the real `polit` binary in a PTY session object, reads the initial screen buffer, sends one key sequence, and verifies the live buffer updates.

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_pty_session -- --nocapture`  
Expected: FAIL because long-lived PTY session support does not exist

**Step 3: Implement minimal reusable PTY session support**

Create a session type that:
- launches the compiled binary
- tracks one child process
- maintains a live `vt100` parser
- supports send keys / type text / resize / read screen
- exposes screen revision increments

Refactor shared PTY code out of `poldev` only where it materially reduces duplication.

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_pty_session -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/mcp/pty_session.rs src/devtools/pty.rs tests/polit_mcp_pty_session.rs
git commit -m "feat(mcp): add live pty session runtime"
```

### Task 3: Implement core MCP control tools

**Files:**
- Modify: `src/bin/polit_mcp.rs`
- Modify: `src/mcp/session.rs`
- Create: `src/mcp/tools.rs`
- Test: `tests/polit_mcp_tools.rs`

**Step 1: Write the failing tool test**

Add tests for:
- `launch`
- `send_keys`
- `read_screen`
- `wait_for_text`
- `terminate`

Each test should validate method-level request/response behavior using the real PTY session machinery where practical.

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_tools -- --nocapture`  
Expected: FAIL because tool handlers are incomplete

**Step 3: Implement the minimal core tools**

Add handlers for:
- `launch`
- `send_keys`
- `read_screen`
- `wait_for_text`
- `terminate`

Responses should include:
- compact summaries
- screen revisions
- bounded visible line output

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_tools -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/polit_mcp.rs src/mcp/session.rs src/mcp/tools.rs tests/polit_mcp_tools.rs
git commit -m "feat(mcp): add core live session tools"
```

### Task 4: Add resize and screenshot support

**Files:**
- Modify: `src/mcp/pty_session.rs`
- Modify: `src/mcp/tools.rs`
- Test: `tests/polit_mcp_visual.rs`

**Step 1: Write the failing visual-support test**

Write tests that:
- resize the active PTY and verify the reported terminal size changes
- request a screenshot and verify an artifact file is created

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_visual -- --nocapture`  
Expected: FAIL because resize/screenshot tooling is incomplete

**Step 3: Implement minimal visual support**

Add:
- `resize`
- `screenshot`

Screenshots should save to a bounded temp artifact location, not flood stdout with binary data.

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_visual -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/mcp/pty_session.rs src/mcp/tools.rs tests/polit_mcp_visual.rs
git commit -m "feat(mcp): add resize and screenshot tools"
```

### Task 5: Add bounded observability helpers

**Files:**
- Create: `src/mcp/inspect.rs`
- Modify: `src/mcp/tools.rs`
- Test: `tests/polit_mcp_inspect.rs`

**Step 1: Write the failing inspect test**

Add tests for:
- `read_save_metadata`
- `read_recent_logs`
- `read_file_excerpt`

The tests should prove the tools are bounded and reject non-whitelisted paths.

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_inspect -- --nocapture`  
Expected: FAIL because inspection helpers do not exist

**Step 3: Implement minimal bounded inspection**

Add:
- whitelisted path families for saves/config/logs
- compact save metadata reader
- bounded log tail reader
- bounded excerpt reader

Reject arbitrary paths and over-large reads.

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_inspect -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/mcp/inspect.rs src/mcp/tools.rs tests/polit_mcp_inspect.rs
git commit -m "feat(mcp): add bounded runtime inspection tools"
```

### Task 6: Validate startup and title interactions through MCP

**Files:**
- Create: `tests/polit_mcp_startup.rs`

**Step 1: Write the failing end-to-end startup test**

Write a test that:
- launches `polit` through the MCP
- confirms the AI setup gate appears on clean HOME
- drives at least one setup interaction
- confirms the next runtime boundary is reached
- terminates cleanly

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_startup -- --nocapture`  
Expected: FAIL because MCP end-to-end control is incomplete

**Step 3: Implement the minimal fixes needed**

Patch only the MCP/session logic required to make the real startup flow controllable and observable end to end.

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_startup -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add tests/polit_mcp_startup.rs
git commit -m "test(mcp): validate startup flow through live session"
```

### Task 7: Add one real gameplay smoke path

**Files:**
- Create: `tests/polit_mcp_gameplay.rs`

**Step 1: Write the failing gameplay smoke test**

Write a test that:
- launches the game
- navigates through startup
- starts a campaign
- submits at least one gameplay action
- confirms the post-action screen updates
- reads bounded save or log metadata
- terminates cleanly

**Step 2: Run test to verify it fails**

Run: `cargo test --test polit_mcp_gameplay -- --nocapture`  
Expected: FAIL because gameplay-level MCP coverage is incomplete

**Step 3: Implement the minimal fixes needed**

Fix only the live control/inspection gaps required for the gameplay smoke path.

**Step 4: Run test to verify it passes**

Run: `cargo test --test polit_mcp_gameplay -- --nocapture`  
Expected: PASS

**Step 5: Commit**

```bash
git add tests/polit_mcp_gameplay.rs
git commit -m "test(mcp): add gameplay smoke path"
```

### Task 8: Document MCP usage for agent-driven playtesting

**Files:**
- Modify: `README.md`
- Create: `docs/polit-mcp.md`

**Step 1: Verify docs are missing**

Run: `rg -n "polit_mcp|playtesting MCP|launch|send_keys|read_screen" README.md docs || true`  
Expected: missing or incomplete references

**Step 2: Write minimal docs**

Document:
- what the MCP is for
- single-session model
- core methods
- observability rules
- example local invocation

**Step 3: Verify docs contain the new usage**

Run: `rg -n "polit_mcp|playtesting MCP|launch|send_keys|read_screen" README.md docs/polit-mcp.md`  
Expected: matching lines found

**Step 4: Commit**

```bash
git add README.md docs/polit-mcp.md
git commit -m "docs(mcp): add live playtesting server usage"
```

### Task 9: Final verification

**Files:**
- Verify only

**Step 1: Run focused MCP and poldev tests**

Run:

```bash
cargo test --test polit_mcp_cli -- --nocapture
cargo test --test polit_mcp_pty_session -- --nocapture
cargo test --test polit_mcp_tools -- --nocapture
cargo test --test polit_mcp_visual -- --nocapture
cargo test --test polit_mcp_inspect -- --nocapture
cargo test --test polit_mcp_startup -- --nocapture
cargo test --test polit_mcp_gameplay -- --nocapture
cargo test --test poldev_scenarios -- --nocapture
cargo test --test poldev_pty_runner -- --nocapture
```

Expected: all pass

**Step 2: Run one manual MCP-backed session smoke test**

Run the `polit_mcp` server locally and verify:
- launch succeeds
- keys advance the runtime
- screen reads stay synchronized
- terminate cleans up

**Step 3: Verify git scope**

Run:

```bash
git status --short
git log --oneline -n 12
```

Expected: only intended MCP changes remain

**Step 4: Commit any final cleanup**

```bash
git add -A
git commit -m "chore(mcp): finalize live playtesting server rollout"
```

If no cleanup is needed, skip this commit.
