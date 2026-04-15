# POLIT

**The American Politics Simulator**

A CLI-based political RPG where you navigate American politics from city council to the Oval Office. Powered by a local AI dungeon master (Gemma 4), an 8-layer mechanistic society simulation, roguelike meta-progression, deckbuilder tactics, and a fully moddable scenario SDK.

Every playthrough is unique. Every consequence is permanent. Every ending earns something.

```
POLIT | Week 34 | Mayor | AP: 4/8
────────────────────────────────────────────────────
  The factory closure in Ward 3 has hit harder than expected.
  Unemployment ticked up to 6.2%, and the union leadership
  is demanding an emergency meeting. Meanwhile, your chief
  of staff just handed you polling numbers -- your approval
  dropped 8 points overnight.

  Fox News is running the story wall-to-wall. The energy
  sector lobbyist who promised campaign funds last week
  just cancelled the meeting.

  > What do you want to do? (/help)
```

---

## Features

### Chat-Forward Interface

Everything flows through the narrative stream. No menus, no spreadsheet UI. You read, you type, you play -- like a conversation with the most complex dungeon master ever built. Floating overlays for maps, cards, laws, and relationships appear when needed and dismiss with Escape.

### AI Dungeon Master

A local Gemma 4 model runs on your hardware -- no internet, no API keys, no cloud. The Narrator adjudicates your freeform actions, voices NPCs with distinct personalities, narrates consequences, and weaves your decisions into emergent storylines. It has 15 tools for affecting the game world but can never override the simulation systems -- it narrates what the systems produce.

**Model tiers:**
| Model | VRAM/RAM | Speed | Quality |
|-------|----------|-------|---------|
| Gemma 4B Q8 | ~3 GB | Instant | Good (lite mode) |
| Gemma 12B Q4 | ~8 GB | 6-10s streaming | Great (recommended) |
| Gemma 27B Q4 | ~17 GB | 6-8s streaming | Excellent |

### 8-Layer Society Simulation

POLIT doesn't use scripted outcomes. It runs a mechanistic simulation of American society where your policy decisions cascade through interconnected systems:

```
Congress passes infrastructure bill
  -> Macro layer: GDP +0.03 via fiscal multiplier, phased over 20 weeks
    -> Sector layer: 9 industry sectors expand output and hiring
      -> Market layer: sector indices rise, bond yields track Fed response
        -> Household layer: disposable income rises, SNAP enrollment shifts
          -> Political layer: approval improves, protest risk drops
            -> Media layer: coverage amplifies positive economic signal
              -> Corporate layer: construction sector donates to your campaign
                -> Geopolitical layer: trade partners adjust bilateral flows
```

| Layer | What it simulates | Tools |
|-------|-------------------|-------|
| **Macro Economy** | GDP, inflation, unemployment, interest rates | Keynesian multipliers, Phillips curve, Okun's law, Taylor rule |
| **Sectors** | 9 industry sectors (Energy, Tech, Finance, etc.) | Mesa ABM -- supply/demand, employment, prices |
| **Financial Markets** | Sector indices, oil/food/metals, 10yr Treasury | ABIDES-inspired price formation, GARCH volatility |
| **Household** | Tax/benefit effects, income by quintile, SNAP/Medicaid | PolicyEngine-inspired microsimulation |
| **Political** | Approval ratings, ideology drift, protest risk, turnout | Mesa voter agents with opinion dynamics |
| **Media** | News amplification, disinformation, scandal coverage | 8 media outlets (CNN, Fox, NYT, etc.) with credibility + reach |
| **Corporate** | Lobbying, donations, retaliation, plant closures | GDD-driven sector interest matrix |
| **Geopolitical** | Trade, sanctions, alliances, migration, conflict | 12 countries (6 Tier 1 + 6 Tier 2), gravity migration model |

The simulation runs in Python via PyO3, called from the Rust game engine once per game week. Real-world data from the Census Bureau and FRED (Federal Reserve) seeds starting conditions.

### Deckbuilder Tactics

Your political toolkit is a deck of cards:

- **Tactic cards** -- what you *do*: Filibuster, Press Conference, Backroom Deal, Rally, Executive Order
- **Asset cards** -- what you *have*: Allies, PACs, Kompromat, Committee Seats, War Chest
- **Position cards** -- what you *believe*: Economic, social, and foreign policy stances that create synergies and vulnerabilities

The **Flip-Flop Mechanic**: your position cards form a coherence score. High coherence makes you "principled" (base trusts you, media respects you, but you're predictable). Low coherence makes you a "flip-flopper" (flexible but trust erodes and attack ads write themselves). Managing this tension is core gameplay.

Cards evolve through use, degrade through neglect, and force hard choices when your deck hits max size.

### Living Law Engine

Draft legislation in plain English. The AI converts it to legal language, identifies mechanical effects, and flags constitutional issues. Laws flow through a 12-stage legislative process:

**Draft -> Sponsor -> Committee -> Floor Rules -> Debate -> Vote -> Conference -> Executive Sign/Veto -> Enforcement -> Challenge -> Court Ruling**

Five enforcement types ensure laws have teeth:
1. **Mechanical** -- modifiers apply directly to simulation (tax rates, spending)
2. **Card-based** -- grant, modify, or restrict cards
3. **Roll-based** -- create DCs for actions (failure = investigation)
4. **AI-interpreted** -- complex law text RAG-queried for legality
5. **Constitutional** -- supreme law; all others checked against it

The Constitution is a living TOML document. Player-passed amendments modify it. Court rulings update interpretations. The law engine evolves through gameplay.

### NPC Social Network

Every character is a node in a petgraph social network with 8 relationship dimensions:

| Dimension | Range | What it means |
|-----------|-------|---------------|
| Trust | -100 to +100 | Will they rely on you? |
| Respect | -100 to +100 | Do they take you seriously? |
| Fear | 0 to 100 | Are they intimidated? |
| Loyalty | 0 to 100 | Will they stick with you? |
| Debt | -10 to +10 | Who owes whom favors? |
| Knowledge | 0 to 100 | How well do they know you? |
| Leverage | 0 to 100 | Do you have dirt on them? |

Reputation propagates through the network at 1-2 hops per week. Allies gossip, rivals leak to journalists, and media amplification can turn a local incident into a national crisis. You have a window to contain damage before it spreads.

NPCs have Big Five personalities, autonomous goal-seeking behavior, career lifecycles, families, and memories of every interaction with you.

### Career Paths

Start anywhere in American government and work your way up -- or sideways, or down:

**Elected offices:** City Council -> Mayor -> State Legislature -> US House -> US Senate -> Vice President -> President

**Bureaucratic careers:** GS-7 Analyst -> Division Chief -> Deputy Director -> Agency Head -> Cabinet Secretary

**Law enforcement:** Beat Cop -> Detective -> DA -> US Attorney -> Attorney General -> FBI Director

**Military:** Officer -> Colonel -> General -> Joint Chiefs -> Secretary of Defense

**Intelligence:** CIA Analyst -> Station Chief -> Deputy Director -> CIA Director

**Judiciary:** Law Clerk -> Federal Judge -> Circuit Court -> Supreme Court (lifetime appointment)

Each path has unique mechanics. Elected officials campaign and fundraise. Bureaucrats navigate budget wars and regulatory capture. Intelligence officers run covert operations that create information entities with exposure risk. Judges issue rulings that permanently change the law engine.

### Information as Weapon

Information exists independently of who knows it. The game tracks:

- **What exists** -- every action creates information entities
- **Who knows** -- knowledge spreads through the social graph
- **What happens when it spreads** -- journalists evaluate newsworthiness, outlets apply editorial lean, public belief shifts

Commands: `/leak`, `/spin`, `/suppress`, `/investigate`, `/plant`, `/deny`, `/confess`

A bribe creates an information entity the moment it happens. It might stay secret forever -- or surface in 20 weeks when your former aide writes a memoir.

### Geopolitics (From Your Desk)

You never *play* as a foreign country. You experience geopolitics as an American politician:

| Office Level | What You See |
|-------------|-------------|
| Local | Headlines, economic effects |
| State | National Guard, trade impacts |
| US House | Vote on war authorization, defense budgets |
| US Senator | Treaty ratification, classified briefings |
| Secretary of State | Direct diplomacy, negotiations |
| President | Nuclear codes, troop deployment, CIA operations |

The war system is political, not tactical. You vote on authorization (your vote follows you forever), manage public opinion during conflict, and deal with the aftermath. Rally-around-the-flag gives a short-term boost that decays rapidly. Casualties erode approval. Veterans become NPCs. Proxy wars drain resources for years.

As President, you have sole nuclear launch authority. The game will let you launch. It will simulate the consequences.

### Corporate System

Corporations exist as political forces, not business simulations. Nine sector blocs (Energy, Tech, Pharma, Defense, Finance, Manufacturing, Agriculture, Healthcare, Retail) react to your policies according to a deterministic interest matrix:

| Your Policy Impact | Corporate Reaction |
|--------------------|--------------------|
| +3 or higher | Major donation, public endorsement |
| +1 to +2 | Quiet donation, favorable coverage |
| -1 to -2 | Lobbyist meeting, donations shift to opponent |
| -3 to -5 | Attack ads, fund primary challenger |
| -5 or worse | Threaten plant closure, legal challenge |

Campaign finance mechanics include PAC donations (limited), Super PAC spending (unlimited, less controllable), and dark money (hard to trace, creates information entities if exposed).

### Elections

Full election simulation from exploratory phase through election night:

1. **Exploratory** -- assess viability, recruit staff, announce
2. **Primary** -- compete within your party, position card tension (primary voters want purity)
3. **General** -- pivot risks, opposition research, debates, ad campaigns, October surprise
4. **Election Day** -- results stream in district-by-district
5. **Aftermath** -- transition or career pivot

Vote calculation uses simulation-grounded inputs: voter ideology distributions, economic conditions, turnout propensity, approval ratings, and issue salience -- all computed by the simulation stack, not hand-tuned numbers.

Presidential elections include the primary gauntlet (Iowa, New Hampshire, Super Tuesday), VP selection, Electoral College (270 to win), and edge cases (269-269 tie, faithless electors, contested results).

### Meta-Progression

Every run ends. Every ending earns something.

- **Legacy Points** -- spent between runs on starting bonuses, narrative seeds, reroll tokens
- **Archetype Unlocks** -- new character types (The Outsider, The Dynasty Heir, The Mogul) unlocked through achievements
- **Card Pool** -- discovered cards appear in future runs
- **Hall of Fame** -- completed characters can appear as NPCs in future playthroughs
- **Dynasty Building** -- your protege runs using your network

Run scoring: Power (highest office x years) + Legacy (lasting impact) + Integrity (coherence + promises kept) + Influence (network size) = S/A/B/C/D/F rating.

**Some unlocks require failure.** Losing gracefully, surviving impeachment, or building a shadow network from outside office are all valid strategies with meta-progression rewards.

### Difficulty Modes

| Mode | Description |
|------|-------------|
| **Story** | Reduced DCs, forgiving NPCs, extra AP, reloads allowed |
| **Standard** | Balanced, single autosave, fair dice |
| **Ironman** | No reloads, harsher consequences, grudges last longer |
| **Nightmare** | Ironman + hostile media + frequent scandals + volatile economy + scheming NPCs |

---

## Technical Architecture

### Engine

- **Runtime:** Rust 2021 edition with bevy_ecs (entity-component-system, no renderer)
- **UI:** Ratatui + Crossterm (terminal UI, 60fps render loop)
- **AI:** Gemma 4 via llama-cpp-2 / ort (ONNX Runtime), local inference
- **Persistence:** RocksDB with column families per domain, snapshot-based saves
- **Scripting:** Rhai (sandboxed, for moddable game logic)
- **Social graph:** petgraph for network computation
- **Audio:** cpal + whisper-rs for local speech-to-text (optional)

### Simulation Stack

Python simulation via PyO3 bridge, feature-gated behind `--features simulation`:

```
Rust (bevy_ecs game loop)
  |
  | PyO3 bridge (MessagePack in, JSON out)
  |
  v
Python simulation host (sim/host.py)
  |
  | 8 layers run sequentially per Dawn Phase tick:
  |
  1. MacroEconomyLayer    -- GDP, inflation, unemployment (Keynesian model)
  2. SectorLayer          -- 9 Mesa SectorAgents (supply/demand/employment)
  3. MarketLayer          -- 13 Mesa MarketAgents (indices, commodities, bonds)
  4. HouseholdLayer       -- tax/benefit microsimulation by quintile
  5. PoliticalLayer       -- 5 Mesa VoterAgents (ideology, turnout, approval)
  6. MediaLayer           -- 8 Mesa MediaAgents (amplification, negativity bias)
  7. CorporateLayer       -- 9 Mesa CorporateAgents (reaction matrix)
  8. GeopoliticalLayer    -- 12 Mesa CountryAgents (trade, migration, conflict)
```

### Data Pipeline

Real-world data seeds game scenarios:

| Source | Data | Used For |
|--------|------|----------|
| **Census Bureau API** | County demographics (ACS 5-year) | Population bootstrap |
| **FRED API** | GDP, unemployment, inflation, Fed rate | Macro starting conditions |
| **BLS** | Employment by sector, wages | Sector calibration |
| **BEA** | GDP by state, personal income | Regional economics |

Five game start modes: **Modern** (latest data), **Historical** (pick a year), **Alternate History** (fork from historical point), **Speculative** (trend projection), **Fictional** (pure TOML scenario).

### Performance

| Component | Target | Actual |
|-----------|--------|--------|
| UI render | 60 fps | 60 fps |
| Simulation tick (8 layers) | < 500ms | ~0.5ms |
| AI inference (12B Q4, M3 Max) | < 10s streaming | 6-10s |
| ECS tick | < 1ms | < 1ms |
| RocksDB read/write | < 1ms | < 1ms |

Hardware requirements:

| Tier | CPU | RAM | GPU | Model |
|------|-----|-----|-----|-------|
| Minimum | 4 cores | 8 GB | None | Gemma 4B Q8 |
| Recommended | 8 cores | 16 GB | Optional 8GB+ VRAM | Gemma 12B Q4 |
| Enthusiast | 8+ cores | 48 GB+ | RTX 4090 / M3 Max | Gemma 27B Q4 |

---

## Modding & SDK

POLIT is fully moddable. Six mod types let you change anything from card balance to entire government systems:

| Mod Type | What You Can Change |
|----------|---------------------|
| **Scenario** | Total conversion -- different government, era, win conditions |
| **System** | Deepen one subsystem (e.g., detailed judicial system) |
| **Content** | Add cards, NPCs, events within existing rules |
| **Department** | Add a specific government entity with full career path |
| **UI** | Custom overlays, commands, status displays |
| **Prompt** | Change AI personality, tone, adjudication style |

### Mod Structure

```
my-mod/
  mod.toml           # manifest: name, type, version, dependencies
  data/              # TOML files: careers, cards, events
  scripts/           # Rhai scripts: custom game logic
  prompts/           # AI prompt overrides
  README.md
```

### Hook Points

40+ hook points across the game lifecycle:

**Lifecycle:** `on_game_start`, `on_dawn_phase`, `on_action_phase`, `on_dusk_phase`, `on_event_trigger`, `on_turn_end`, `on_career_end`

**Systems:** `on_law_proposed`, `on_law_enacted`, `on_election_called`, `on_vote_cast`, `on_npc_action`, `on_relationship_change`, `on_economic_tick`, `on_card_played`, `on_dice_roll`, `on_crisis_start`, `on_war_declared`

**Entities:** `on_npc_spawn`, `on_npc_death`, `on_promotion`, `on_corp_react`, `on_foreign_event`

**UI:** `register_overlay`, `register_command`, `register_status`, `inject_briefing`

**AI:** `extend_tools`, `extend_context`, `register_mode`, `modify_prompt`

### Rhai Scripting API

36 functions for modding game logic:

```rhai
// Grant a card when a law passes
fn on_law_enacted(law) {
    if law.type == "tax_reform" {
        grant_card(get_player(), "Tax Expert");
        narrate("Your mastery of the tax code earns recognition.");
    }
}

// Custom event with consequences
fn on_game_start() {
    schedule_event(12, "factory_closure", #{
        region: "Midwest",
        severity: 0.6,
    });
}
```

### Scenario TOML

Define starting conditions for any historical or fictional scenario:

```toml
[scenario]
name = "The Great Recession"
description = "Navigate the 2008 financial crisis"
era = "historical"
year = 2008

[macro]
gdp_growth = -0.028
inflation = 0.001
unemployment = 0.073
fed_funds_rate = 0.01
consumer_confidence = 55.0
debt_to_gdp = 0.68

[geopolitical.overrides.Russia]
alignment = -0.3
stability = 60

[[events.scheduled]]
week = 5
type = "EconomyShock"
shock_type = "banking_crisis"
magnitude = 4.0

[[events.scheduled]]
week = 15
type = "FiscalBill"
bill_type = "spending"
amount_gdp_pct = 0.05
```

### SDK CLI

```bash
polit-sdk new my-scenario          # scaffold a new mod
polit-sdk validate my-scenario/    # check for errors
polit-sdk test my-scenario/        # run mod tests
polit-sdk package my-scenario/     # build distributable
```

---

## Getting Started

### Requirements

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Python 3.11+ (for simulation stack)
- ~8 GB free disk space (model download)
- Terminal with 80x24 minimum (120x40 recommended)

### Install

```bash
git clone https://github.com/shinyobjectz/polit.git
cd polit
make venv          # set up Python simulation environment
make quick         # build + install to PATH
polit              # launch
```

On first launch, POLIT auto-detects your hardware and downloads the appropriate Gemma model from Hugging Face.

### Development

```bash
make quick         # build + install to PATH (fast iteration)
make update        # fmt + lint + test + install (before pushing)
make test          # Rust unit + integration tests
make sim-test      # Python simulation tests (233 tests)
make sim-bench     # simulation tick benchmark
make run           # launch without installing
```

### TUI Validation

`poldev` drives POLIT's `ratatui` runtime through checked-in YAML scenarios.

```bash
cargo run --bin poldev -- tui run tests/tui/scenarios/first_launch_ai_setup_codex.yaml
cargo run --bin poldev -- tui run --mode pty tests/tui/scenarios/first_launch_ai_setup_codex.yaml
```

- `in_process` runs the current runtime in-process with deterministic frame capture.
- `pty` launches the real compiled `polit` binary inside a pseudo-terminal.
- Scenarios live under `tests/tui/scenarios/`.
- Failure output includes the scenario name, backend mode, failing step, temp home path, recent input, and the last frame dump.

More detail: [docs/poldev.md](docs/poldev.md)

### Live Playtesting MCP

`polit_mcp` exposes a single live `polit` session over stdio so an agent can playtest the real binary turn by turn.

```bash
cargo build --bin polit --bin polit_mcp
target/debug/polit_mcp
```

- launches the real compiled `polit` binary inside a PTY
- sends keyboard input with `send_keys`
- reads bounded visible terminal text with `read_screen` and `wait_for_text`
- supports bounded save/config/log inspection under `~/.polit/`

More detail: [docs/polit-mcp.md](docs/polit-mcp.md)

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `CENSUS_API_KEY` | Census Bureau API key for real population data |
| `FRED_API_KEY` | FRED API key for real economic data |
| `HF_TOKEN` | Hugging Face token for model download |
| `PYO3_PYTHON` | Path to Python 3.12 (macOS Homebrew: `/opt/homebrew/bin/python3.12`) |

---

## Game Scenarios

POLIT ships with pre-built scenarios seeded from real data:

| Scenario | Era | Starting Conditions |
|----------|-----|---------------------|
| **Modern USA 2024** | Modern | Current economy, China/Russia rivalry, strong NATO |
| **Great Recession 2008** | Historical | Banking crisis, near-zero rates, collapsing confidence |
| **Cold War Tension** | Stylized | Multi-front geopolitical stress, Iran conflict, Russia sanctions |
| **Boom Economy** | Historical | Late-90s tech boom, low unemployment, dot-com wobble |
| **1970s Stagflation** | Historical | Oil crisis, 9% inflation, negative growth, OPEC shock |

Create your own with a TOML file in `game/scenarios/` or generate one from real FRED data:

```bash
polit-data fetch --year 2016 --output game/scenarios/election_2016.toml
```

---

## Design Philosophy

Six pillars guide every design decision:

1. **Everything is systems, not scripts** -- emergent outcomes from interacting simulations, never hard-coded drama
2. **Consequences are real and permanent** -- social graph has memory, economy has inertia, laws persist
3. **Every playthrough is unique** -- procedural generation ensures no two runs alike
4. **Depth without complexity walls** -- 5-minute onboarding, depth reveals through play
5. **The AI is dungeon master, not game designer** -- narrates and adjudicates, never overrides systems
6. **Failure is content** -- losing is a story, meta-progression rewards all endings

---

## Project Structure

```
polit/
  src/
    engine/          # ECS world, game loop, channels, persistence
    systems/         # Economy, social graph, cards, dice, simulation bridge
    ai/              # Agent, context builder, tool router, inference
    ui/              # Ratatui app, character creation, chat, overlays
    persistence/     # RocksDB interface
    scripting/       # Rhai integration
  sim/
    layers/          # 8 simulation layers (Python/Mesa)
    agents/          # Mesa agent definitions
    models/          # Data models (County, Household)
    bootstrap/       # Population seeding from Census API
    data_pipeline/   # FRED API fetcher
    tests/           # 233 Python tests
  game/
    scenarios/       # Scenario TOML files
    config/          # Balance, theme, difficulty
  docs/
    gdd/             # 21-section game design document
```

---

## License

[TBD]

---

*POLIT is a solo passion project. If you're interested in contributing, open an issue to discuss.*
