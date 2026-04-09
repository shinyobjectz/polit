---
title: Modding & Scenario SDK
section: 11
status: design-complete
depends_on: [01, 10]
blocks: []
---

# Modding & Scenario SDK

## Mod Types

| Type | Scope | Example |
|------|-------|---------|
| Scenario | Total conversion — replaces everything | "Corporate Republic", "Parliamentary UK" |
| System | Deepens one subsystem | "Deep Economics", "Military Realism" |
| Content | Adds data within existing rules | "1920s Prohibition Era" scenario |
| Department | Deepens a specific government entity | "FBI Deep Dive" — case investigation mechanics |
| UI | Visual/interaction changes | "Political Map Pro" — enhanced map overlays |
| Prompt | AI behavior changes | "Veep Mode" — satirical tone |

## Mod Manifest

```toml
# mods/fbi_deep_dive/mod.toml

[mod]
name = "FBI Deep Dive"
type = "department"
version = "1.2.0"
author = "modder_name"
description = "Full FBI career simulation"
compatible_scenarios = ["modern_usa"]
min_engine_version = "0.5.0"

[mod.systems]
extends = ["law_enforcement"]
adds = ["investigation", "interrogation", "evidence", "fisa_court"]
replaces = []

[mod.dependencies]
requires = []
conflicts = ["cia_deep_dive < 2.0"]
```

## Mod File Structure

```
mods/fbi_deep_dive/
├─ mod.toml              manifest
├─ data/
│  ├─ career_path.toml   FBI-specific ranks
│  ├─ case_types.toml    investigation types
│  ├─ evidence_rules.toml
│  └─ cards/             FBI-specific cards
├─ scripts/
│  ├─ investigation.rhai
│  ├─ interrogation.rhai
│  └─ fisa_court.rhai
├─ prompts/
│  ├─ fbi_npc_templates/
│  └─ case_narration.toml
└─ README.md
```

## Total Conversion Example

```toml
# scenarios/corporate_republic/scenario.toml

[meta]
name = "Corporate Republic"
description = "The Fortune 500 ARE the government."
author = "modder_handle"
version = "1.0.0"
base_year = 2030
era_type = "speculative"

[government]
type = "corporate_oligarchy"
levels = ["board", "division", "subsidiary", "franchise"]
executive = "CEO-President"
legislature = "Board of Directors (50 seats, weighted by market cap)"
judiciary = "Arbitration Council"

[win_conditions]
primary = "Achieve CEO-President with 60% board control"
alternate = [
  "Overthrow corporate system, restore democracy",
  "Build worker cooperative controlling 3+ divisions",
]

[custom_systems]
scripts = [
  "scripts/hostile_takeover.rhai",
  "scripts/stock_manipulation.rhai",
]

[card_overrides]
replace = [
  { from = "Filibuster", to = "Hostile Takeover Bid" },
  { from = "Rally Base", to = "Shareholder Meeting" },
]
add = ["Stock Buyback", "Golden Parachute", "Whistleblower"]
```

## Hook Points

### Lifecycle Hooks
| Hook | When |
|------|------|
| `on_game_start` | Scenario initialization |
| `on_dawn_phase` | Beginning of each week |
| `on_action_phase` | Before player acts |
| `on_dusk_phase` | End of each week |
| `on_event_trigger` | Any event fires |
| `on_turn_end` | After dusk, before next dawn |
| `on_career_end` | Run ending |
| `on_save` / `on_load` | Persistence moments |

### System Hooks
| Hook | When |
|------|------|
| `on_law_proposed` | Legislation enters pipeline |
| `on_law_enacted` | Law passes all stages |
| `on_law_enforced` | Enforcement check triggered |
| `on_election_called` | Election cycle begins |
| `on_vote_cast` | Floor vote happens |
| `on_npc_action` | NPC takes autonomous action |
| `on_relationship_change` | Social graph edge modified |
| `on_info_created` | New information entity |
| `on_info_published` | Information goes public |
| `on_economic_tick` | Economy model steps |
| `on_card_played` | Card used |
| `on_card_acquired` | New card gained |
| `on_dice_roll` | Any roll happens |
| `on_crisis_start` | Crisis phase begins |
| `on_war_declared` | Military conflict starts |
| `on_custom_action` | Freeform action pipeline fires |

### Entity Hooks
`on_npc_spawn`, `on_npc_death`, `on_npc_retire`, `on_promotion`, `on_corp_react`, `on_foreign_event`

### UI Hooks
`register_overlay`, `register_command`, `register_status`, `inject_briefing`, `inject_narrative`

### AI Hooks
`extend_tools`, `extend_context`, `register_mode`, `modify_prompt`

## Rhai Scripting API

```rust
// World state
get_var(name)                    // read simulation variable
set_var(name, value)             // modify simulation variable
get_week()                       // current game week
get_era()                        // current era/year

// Characters
get_player()                     // player entity data
get_npc(name_or_id)              // NPC entity data
spawn_npc(template)              // create new NPC
modify_stat(entity, stat, delta) // change character stat
modify_rel(a, b, field, delta)   // change relationship edge

// Cards
grant_card(entity, card_id)      // give a card
revoke_card(entity, card_id)     // remove a card
create_card(definition)          // mint new card type
check_card(entity, card_id)      // does entity have card?

// Events
trigger_event(event_id)          // fire an event
schedule_event(event_id, weeks)  // queue future event
create_event(definition)         // define new event type

// Dice
roll(sides)                      // roll a die
roll_check(skill, dc)            // skill check with modifiers

// Economy
get_econ(indicator)              // read economic variable
set_econ(indicator, value)       // modify economic variable

// Laws
enact_law(definition)            // force-enact a law
repeal_law(id)                   // remove active law
check_legal(action)              // is this legal?

// Narration
narrate(text)                    // display text to player
narrate_styled(text, style)      // with formatting
prompt_choice(options)           // present choices

// Meta
log(message)                     // debug logging
save_custom_data(key, value)     // persist mod data
load_custom_data(key)            // retrieve mod data
```

## SDK CLI

| Command | Purpose |
|---------|---------|
| `polit-sdk new <name> --type <type>` | Scaffold mod by type |
| `polit-sdk validate <path>` | Check mod integrity |
| `polit-sdk test <path>` | Headless simulation test |
| `polit-sdk test <path> --hooks` | Verify hooks fire correctly |
| `polit-sdk package <path>` | Bundle for distribution |
| `polit-sdk publish <path>` | Publish to mod registry |
| `polit-sdk install <name>` | Install from registry |
| `polit-sdk list` | List installed mods |
| `polit-sdk enable/disable <name>` | Toggle mods |
| `polit-sdk conflicts` | Check for mod conflicts |
| `polit-sdk inspect <name> --hooks` | Show hook usage |
| `polit-sdk bench <path>` | Performance impact |
| `polit-sdk docs <path>` | Generate documentation |
| `polit-sdk migrate <path> --to <ver>` | Update for new engine |
