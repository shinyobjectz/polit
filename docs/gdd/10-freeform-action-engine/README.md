---
title: Freeform Action Engine
section: 10
status: design-complete
depends_on: [01, 04, 06]
blocks: [11]
---

# Freeform Action Engine

## Core Principle

The player can type ANY action. The DM never says "you can't do that." It says "you can TRY — here's what happens."

## Action Classification Pipeline

Player input goes through classification:

1. **KNOWN ACTION** — maps to existing system (e.g., "meet Sen. Davis" → standard `/meet` flow)
2. **CUSTOM ACTION** — no system exists, DM builds one in real-time
3. **IMPOSSIBLE** — physically/logically impossible. DM plays it for humor or finds the metaphor.

## Custom Action Pipeline

When the DM encounters a custom action:

### Step 1: Classify Severity
- Legal risk (none → extreme)
- Career risk (none → terminal)
- Moral weight
- Reversibility
- Complexity

### Step 2: Build Custom Framework
DM generates a structured action chain using its tool suite:

```json
{
  "action": "assassination_plot",
  "phases": [
    {
      "name": "Planning",
      "rolls": [{ "skill": "Cunning", "dc": 18 }],
      "ap_cost": 3,
      "cards_relevant": ["Fixer", "Kompromat"]
    },
    {
      "name": "Execution",
      "rolls": [
        { "skill": "Deception", "dc": 20 },
        { "skill": "Luck", "dc": 16 }
      ],
      "consequences": {
        "success": "target_eliminated, guilt_tracker_starts",
        "partial": "target_survives, investigation",
        "failure": "caught, career_terminal"
      }
    },
    {
      "name": "Aftermath",
      "ongoing": true,
      "systems": ["investigation_tracker", "guilt_stress_modifier"]
    }
  ]
}
```

### Step 3: Persist as Custom System
Generated framework saved to RocksDB as `custom_event_schema`. Becomes a REAL system with ongoing consequences.

### Step 4: Narrate and Play
DM presents it in the chat-forward flow with options to proceed, back out, or ask more.

## Custom Action Examples

| Player Input | Generated System |
|-------------|-----------------|
| "Assassinate the governor" | Multi-phase plot with planning, execution, aftermath. Investigation tracker, guilt stress. |
| "Get drunk at the fundraiser" | Social check DC 10 for composure. Failure tiers: embarrassing quote → viral video → assault charge. |
| "Start a secret political society" | Multi-week arc. Recruit members, establish structure, weekly exposure checks. |
| "Seduce Sen. Martinez" | Charisma checks. If married: affair mechanics with discovery risk. Relationship/catastrophe fork. |
| "Learn martial arts" | Low-stakes ongoing. 1 AP/week, benefits after 8 weeks. +2 Intimidation, stress reduction. |

## Self-Extending SDK

When Gemma 4 generates a custom event framework, it can:

1. **Persist the schema** to RocksDB — becomes a real game system
2. **Generate Rhai scripts** for complex ongoing logic — hooks into Dawn Phase tick
3. **Register new card types** — "Hitman Contact" asset card enters the game permanently
4. **Chain into future events** — assassination plot spawns "Guilt" chain, "Cold Case Reopened" 20 weeks later
5. **Cross-run persistence** — if a custom event schema proves interesting (player engaged 3+ turns), flagged for permanent event pool. Future runs may encounter it.

### Safety Rails
- DM-generated Rhai scripts are sandboxed
- Scripts can only call approved game APIs
- Max script size enforced
- Scripts validated before execution
- Player can `/inspect` any custom system to see its rules

## DM Decision Framework for Custom Actions

For ANY player input not matching a known system:

1. **IS IT POSSIBLE?** Yes → proceed. No but fun → closest interpretation. Truly impossible → humor/character.
2. **WHAT DOES IT COST?** AP, money, relationships, cards — proportional to complexity.
3. **WHAT COULD GO WRONG?** Map failure modes to consequence tiers. Always at least 2 tiers.
4. **WHAT COULD GO RIGHT?** Tangible game benefits. Never just flavor — always mechanical impact.
5. **HOW LONG DOES IT LAST?** One-shot, multi-phase, or ongoing with weekly upkeep.
6. **DOES IT GENERATE NARRATIVE?** Custom events create STORY. Other characters react.

## Slash Commands

| Command | Action |
|---------|--------|
| `/propose <description>` | Ask DM to build a custom event |
| `/inspect <event>` | See mechanical framework behind any active event |
| `/abandon <event>` | Walk away from custom event chain (consequences apply) |
| `/journal` | See all active custom events and status |
