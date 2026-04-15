# AI Provider Setup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a first-launch AI setup wizard that lets players choose Codex Subscription or OpenRouter, stores only non-secret provider config on disk, stores OpenRouter credentials in secure storage, and routes all runtime AI through the configured provider.

**Architecture:** Replace the hardcoded local-model boot path with a provider configuration layer plus a provider factory. The setup wizard owns first-launch validation and persistence, while `CodexProvider` and `OpenRouterProvider` both adapt into the existing `AiProvider` trait so character creation and the main game loop share the same runtime interface.

**Tech Stack:** Rust 2021, Ratatui, Crossterm, existing `AiProvider` abstraction, `reqwest`, `serde`, `toml`, OS credential storage crate, local `codex` CLI integration

---

### Task 1: Add AI config types and setup-state detection

**Files:**
- Create: `src/ai/config.rs`
- Modify: `src/ai/mod.rs`
- Modify: `src/engine/paths.rs`
- Test: `src/ai/config.rs`

**Step 1: Write the failing config tests**

Add tests for:

- missing `ai.toml` reports setup required
- valid persisted provider metadata loads successfully
- config serialization excludes secret fields

**Step 2: Run tests to verify they fail**

Run: `cargo test ai::config`
Expected: FAIL with unresolved module or missing config loader

**Step 3: Write minimal implementation**

Implement:

- `AiProviderKind`
- `AiSetupState`
- `AiConfig`
- `AiConfig::load(...)`
- `AiConfig::save(...)`
- `AiConfig::setup_required(...)`

Seed `~/.polit/config/ai.toml` only when setup completes; do not create fake default provider state.

**Step 4: Run tests to verify they pass**

Run: `cargo test ai::config`
Expected: PASS

### Task 2: Add secure credential storage abstraction

**Files:**
- Create: `src/ai/secrets.rs`
- Modify: `Cargo.toml`
- Modify: `src/ai/mod.rs`
- Test: `src/ai/secrets.rs`

**Step 1: Write the failing credential-storage tests**

Add tests for:

- OpenRouter key save and load round-trip through an abstract storage trait
- config file payload never contains the API key
- unavailable secure storage returns a blocking error

**Step 2: Run tests to verify they fail**

Run: `cargo test ai::secrets`
Expected: FAIL with unresolved module or missing storage abstraction

**Step 3: Write minimal implementation**

Add:

- a secure-storage trait
- a production implementation backed by an OS keychain crate
- a test double for unit tests
- explicit errors for unavailable or failed secure storage

Do not add plaintext fallback.

**Step 4: Run tests to verify they pass**

Run: `cargo test ai::secrets`
Expected: PASS

### Task 3: Add Codex provider adapter and validation

**Files:**
- Create: `src/ai/codex.rs`
- Modify: `src/ai/mod.rs`
- Test: `src/ai/codex.rs`

**Step 1: Write the failing Codex tests**

Add tests for:

- `codex` binary missing returns a dedicated validation error
- unauthenticated or unusable Codex health check returns a setup-blocking error
- a valid Codex response maps into `DmResponse`

**Step 2: Run tests to verify they fail**

Run: `cargo test ai::codex`
Expected: FAIL with unresolved module or missing provider

**Step 3: Write minimal implementation**

Implement:

- `CodexProvider`
- startup validation for local Codex availability
- a request path that shells out to the supported local Codex client boundary
- response normalization into `DmResponse`

Keep all Codex-specific process handling inside this module.

**Step 4: Run tests to verify they pass**

Run: `cargo test ai::codex`
Expected: PASS

### Task 4: Add OpenRouter provider adapter and validation

**Files:**
- Create: `src/ai/openrouter.rs`
- Modify: `src/ai/mod.rs`
- Test: `src/ai/openrouter.rs`

**Step 1: Write the failing OpenRouter tests**

Add tests for:

- missing model returns validation error
- missing API key returns validation error
- successful validation request constructs a provider
- OpenRouter response content maps into `DmResponse`

**Step 2: Run tests to verify they fail**

Run: `cargo test ai::openrouter`
Expected: FAIL with unresolved module or missing provider

**Step 3: Write minimal implementation**

Implement:

- `OpenRouterProvider`
- blocking validation request using `reqwest`
- request and response mapping into existing AI runtime types

Keep the OpenRouter key lookup inside the secure-storage abstraction.

**Step 4: Run tests to verify they pass**

Run: `cargo test ai::openrouter`
Expected: PASS

### Task 5: Add provider factory and runtime bootstrap wiring

**Files:**
- Create: `src/ai/factory.rs`
- Modify: `src/ai/mod.rs`
- Modify: `src/main.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/engine/mod.rs`
- Test: `tests/e2e_flow.rs`

**Step 1: Write the failing bootstrap tests**

Add coverage proving:

- startup with valid AI config uses a real configured provider instead of mock AI
- the same configured provider type is used for character creation and the main game

**Step 2: Run tests to verify they fail**

Run: `cargo test --test e2e_flow`
Expected: FAIL with hardcoded provider boot path still active

**Step 3: Write minimal implementation**

Implement:

- `ConfiguredAiProviderFactory`
- `build_provider_for_runtime(...)`
- `build_provider_for_character_creation(...)`

Remove hardcoded Gemma loading from `src/main.rs` and `src/ui/mod.rs`.

Do not leave any implicit mock fallback in production startup.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test e2e_flow`
Expected: PASS

### Task 6: Add first-launch setup wizard UI

**Files:**
- Create: `src/ui/setup.rs`
- Modify: `src/ui/mod.rs`
- Modify: `src/ui/title.rs`
- Test: `src/ui/setup.rs`

**Step 1: Write the failing setup-screen tests**

Add tests for:

- missing or invalid config routes into setup
- selecting Codex runs Codex validation and persists provider metadata on success
- selecting OpenRouter requires model plus secure key save and validation
- validation failure keeps the user in setup

**Step 2: Run tests to verify they fail**

Run: `cargo test ui::setup`
Expected: FAIL with unresolved module or missing setup flow

**Step 3: Write minimal implementation**

Implement a blocking setup wizard that:

- appears on first launch before the title screen
- offers Codex Subscription and OpenRouter
- captures OpenRouter model and API key
- writes `ai.toml` plus secure key storage on success
- reopens from `TitleAction::Settings`

Keep the form scope tight. Do not bundle unrelated settings into this screen.

**Step 4: Run tests to verify they pass**

Run: `cargo test ui::setup`
Expected: PASS

### Task 7: Add broken-provider recovery flow

**Files:**
- Modify: `src/ui/mod.rs`
- Modify: `src/ai/factory.rs`
- Test: `tests/game_loop.rs`

**Step 1: Write the failing recovery tests**

Add coverage proving:

- previously saved but now invalid provider state routes back into setup or recovery
- the app does not silently continue with mock AI

**Step 2: Run tests to verify they fail**

Run: `cargo test --test game_loop`
Expected: FAIL with startup still bypassing recovery behavior

**Step 3: Write minimal implementation**

Add:

- provider construction error mapping for UI
- a recovery path that sends the user back into setup

Do not continue into character creation or game start until provider recovery succeeds.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test game_loop`
Expected: PASS

### Task 8: Update docs and provider-facing defaults

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/plans/2026-04-15-ai-provider-setup-design.md`

**Step 1: Write the failing documentation checklist**

Record the required doc changes:

- Gemma-first startup assumptions removed
- first-launch AI setup documented
- Codex local prerequisite documented
- OpenRouter secure credential behavior documented

**Step 2: Perform minimal documentation updates**

Update runtime docs to match the implemented provider flow and remove outdated “always local Gemma” claims.

**Step 3: Verify documentation consistency**

Run: `rg -n "Gemma|HF_TOKEN|local model|OpenRouter|Codex" README.md AGENTS.md docs/plans/2026-04-15-ai-provider-setup-design.md`
Expected: Only current, accurate provider guidance remains

### Task 9: Run full verification and land the work

**Files:**
- Verify only

**Step 1: Run focused AI and UI tests**

Run: `cargo test ai::config ai::secrets ai::codex ai::openrouter ui::setup`
Expected: PASS

**Step 2: Run integration verification**

Run: `cargo test --test e2e_flow --test game_loop`
Expected: PASS

**Step 3: Run broader project verification**

Run: `cargo test`
Expected: PASS

**Step 4: Update tracker and commit**

```bash
bd close polit-atfh
bd sync
git add Cargo.toml README.md AGENTS.md src/ai src/ui/setup.rs src/ui/mod.rs src/ui/title.rs src/main.rs src/engine/mod.rs tests/e2e_flow.rs tests/game_loop.rs docs/plans/2026-04-15-ai-provider-setup-design.md docs/plans/2026-04-15-ai-provider-setup-plan.md
git commit -m "feat(ai): add first-launch provider setup"
```
