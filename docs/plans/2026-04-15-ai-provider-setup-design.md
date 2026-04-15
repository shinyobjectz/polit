# AI Provider Setup Design

**Date:** 2026-04-15
**Issue:** `polit-atfh`

## Goal

Add a first-launch setup flow that requires POLIT to configure a working AI provider before any AI-backed gameplay begins. The setup must support:

- `Codex Subscription` via a required local `codex` client and authenticated local session
- `OpenRouter` via a user-selected model plus a securely stored API key

The selected provider must power all runtime AI interactions, including character creation and the main game loop.

## Current Verified State

- Startup still assumes a local Gemma provider in `src/main.rs` and `src/ui/mod.rs`.
- `TitleAction::Settings` exists, but selecting it currently exits instead of opening a real settings flow.
- Character creation already accepts a boxed `AiProvider` and runs it on a background thread.
- The main game loop already accepts a boxed `AiProvider` via `GameState::with_provider(...)`.
- Config and save data already resolve under `~/.polit` using `GamePaths`.

## External Constraint

OpenAI’s current Codex docs indicate that Codex subscription access is available through Codex clients and SDKs, and the supported embedding path is the local Codex client or SDK surface rather than treating a ChatGPT subscription as a normal API key. This design therefore treats `Codex Subscription` as a local runtime dependency, not a token field inside POLIT.

Sources:

- [OpenAI for developers](https://developers.openai.com/)
- [Codex SDK](https://developers.openai.com/codex/sdk)

## Product Behavior

### First Launch

On startup, POLIT checks for valid AI provider configuration under `~/.polit/config`.

- If configuration is missing or invalid, POLIT opens a blocking setup wizard before the title screen.
- The wizard cannot be skipped because AI is now mandatory for gameplay.
- Setup completion requires a real validation pass against the selected provider.

### Setup Choices

The setup wizard presents two provider choices:

1. `Codex Subscription`
2. `OpenRouter`

#### Codex Subscription path

- Detect a local `codex` binary in `PATH`
- Verify that the local Codex installation is authenticated and usable
- Persist non-secret provider metadata, such as provider kind and preferred model
- Do not store secrets in POLIT for this path

If Codex is missing or unauthenticated, the wizard stays on the Codex branch and shows an actionable error with the exact local prerequisite that failed.

#### OpenRouter path

- Prompt for model identifier
- Prompt for API key
- Store the API key only via secure OS-backed credential storage
- Persist only non-secret metadata in config files
- Validate the configured model and credentials with a real provider call before setup completes

If secure credential storage is unavailable, POLIT must fail closed. It must not silently write the API key in plaintext to `~/.polit/config`.

### Runtime Use

The chosen provider must be used for:

- character creation AI
- main game narration
- tool-call generation
- future AI entry points that already use the shared `AiProvider` abstraction

There is no fallback to mock AI once setup has completed.

### Recovery Behavior

If a previously configured provider becomes invalid later:

- startup should detect that the provider cannot be constructed or validated
- POLIT should route the player into a recovery or setup screen
- POLIT should not proceed into partial gameplay with broken AI state

## Technical Design

### Provider factory

Introduce a runtime provider factory that owns:

- loading persisted AI settings
- loading secrets from secure storage where applicable
- validating provider prerequisites
- constructing a boxed `AiProvider`

This removes the current hardcoded Gemma boot path from startup and from the transition into the live game.

### Provider modules

Split provider logic into explicit backends:

- `CodexProvider`
- `OpenRouterProvider`

Both backends adapt their raw response shape into the existing `DmResponse` contract so the rest of the runtime does not need backend-specific handling.

### Config layout

Add a dedicated AI config file under `~/.polit/config/ai.toml`.

This file should store:

- selected provider kind
- default model name for the selected provider
- last validation timestamp or state
- any provider-specific non-secret settings

This file must not store:

- OpenRouter API keys
- Codex auth secrets

### Secure storage

Use platform secure storage for the OpenRouter API key. Typical targets are:

- macOS Keychain
- Windows Credential Manager
- Linux Secret Service, when available

If the platform integration fails or is unavailable, POLIT should surface a blocking error for the OpenRouter path rather than downgrading security.

### Startup flow changes

The runtime flow becomes:

1. Initialize `GamePaths`
2. Load AI setup state
3. If setup incomplete or invalid, run setup wizard
4. Build the configured `AiProvider`
5. Continue into title screen or new-game flow
6. Reuse the same provider factory when entering the game loop so the same configured backend is used consistently

### Settings entry

The existing `Settings` title action should reopen the AI setup flow later. First-launch setup remains the primary entry point, but the settings route allows provider switching and recovery.

## Error Handling

### Codex errors

The setup wizard should distinguish:

- `codex` binary missing
- Codex installed but unauthenticated
- Codex health check failed
- Codex request failed during validation

### OpenRouter errors

The setup wizard should distinguish:

- missing model
- missing API key
- secure storage unavailable
- secure storage write failure
- authentication failure
- model or network validation failure

All of these should remain inside setup and preserve any user-entered non-secret form state when practical.

## Testing Strategy

Testing should focus on runtime-production behavior:

- missing config triggers setup
- valid persisted config resolves the correct provider type
- invalid config or unavailable prerequisites return the app to setup
- setup success persists only non-secret config
- setup never stores OpenRouter keys in plaintext config files
- both character creation and the main game loop use the configured provider path
- provider adapters normalize backend output into `DmResponse`

## Out of Scope

This pass should not add:

- multi-provider failover
- plaintext credential fallback
- cloud-hosted Codex auth inside POLIT
- generalized settings overhaul beyond the provider setup and recovery path
