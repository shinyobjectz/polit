---
title: Character Creation & Meta-Progression
section: 08
status: design-complete
depends_on: [01, 03]
blocks: [20]
---

# Character Creation & Meta-Progression

## Character Creation Flow

### 1. Scenario Select
- Modern (2024+)
- Historical (year picker + event context)
- Alternate History (fork point + divergence prompt)
- Speculative (future extrapolation settings)
- Custom Scenario (mod-defined)

### 2. Archetype Select (unlocked through meta-progression)

| Archetype | Starting Strengths | Starting Weaknesses |
|-----------|-------------------|---------------------|
| The Idealist | Strong position cards | Weak asset cards |
| The Machine | Strong asset cards | Flexible/weak positions |
| The Outsider | High charisma, wildcard events | No political cards |
| The Dynasty Heir | Strong network | High expectations, family baggage |
| The Dealmaker | Negotiation bonuses, debts on both sides | Low trust base |
| The Prosecutor | Legal knowledge, investigation tools | Narrow initial network |
| The Activist | Grassroots cards, media savvy | No institutional access |
| The Veteran | Military background, FP bonuses | Civilian politics unfamiliar |
| The Mogul | Massive war chest, business connections | Zero political experience |
| [LOCKED] | Unlock through achievements | — |

### 3. Background Builder
- **Origin**: Hometown (affects starting district/state)
- **Education**: High school → law/business/military/academic
- **Career**: Pre-politics profession (sets starting stats)
- **Family**: Married? Kids? Political family?
- **Traits**: Pick 2 positive, 1 negative
  - Positive: Charismatic, Policy Wonk, Ruthless, Connected, Resilient, Photogenic, Silver Tongue, Strategic Mind
  - Negative: Skeleton in Closet, Hot Temper, Health Issues, Spending Problem, Loose Cannon, Trust Issues
- Gemma 4 generates a backstory paragraph weaving all selections

### 4. Starting Position
- **Entry level**: City council, school board, county clerk
- **Mid entry** (requires prereqs): State legislator, mayor, DA
- **High entry** (rare archetypes only): US House, state AG, media personality
- **Bureaucratic entry**: GS-7 analyst, beat cop, military officer, CIA analyst

### 5. Tone Configuration
- Narrative dial: Gritty Realism ←→ Political Satire
- Difficulty: Story Mode / Standard / Ironman / Nightmare
- Historical accuracy: Loose / Moderate / Strict
- Writes to `prompts/tone.toml` for the session

## Meta-Progression — Legacy System

### Permanent Unlocks (never lost)
- **Archetypes**: Complete specific achievements to unlock
- **Card pool expansions**: Discovered cards added to master pool
- **Scenario unlocks**: Milestones unlock historical scenarios and alt-history forks
- **Cosmetic**: Title cards, character portraits, UI themes

### Legacy Points (spent between runs)
- Earned from run achievements (not just winning):
  - Passed X laws, built coalition of Y, survived scandal, lost gracefully, historic achievement
- Spent on:
  - Starting bonuses (extra AP week 1, bonus card draw)
  - Narrative seeds ("start with a mentor NPC who knows you")
  - Reroll tokens (limited per run)
  - Background perks
- Points add OPTIONS, not power

### Hall of Fame
- Every completed run gets a summary card
- Hall of Fame characters can appear as NPCs in future runs
- Political dynasty builds across runs

### Run Scoring

| Score | Basis |
|-------|-------|
| Power | Highest office × years held |
| Legacy | Lasting impact (laws active, appointments serving) |
| Integrity | Coherence + promises kept + clean record |
| Influence | Size of network at career end |
| **Overall** | **Combined into S/A/B/C/D/F Legacy Rating** |

## Career End Conditions

### Win Conditions
- Reach target office (player-set goal)
- Serve full term as President
- Supreme Court appointment
- Pass constitutional amendment
- Build lasting dynasty (3+ proteges in office)
- Custom (mod-defined)

### Loss Conditions
- Lose election with no comeback path
- Impeached and removed
- Criminal conviction
- Health crisis / death
- Total reputation collapse (approval < 5% everywhere)
- Resign in disgrace

### Neutral Endings
- Voluntary retirement
- Career pivot to private sector
- Age out

**All endings earn meta-progression rewards. "How you fall matters as much as how high you climb."**

## Difficulty Modes

| Mode | Description |
|------|-------------|
| Story | Reduced DCs, forgiving NPCs, extra AP, reloads allowed |
| Standard | Balanced challenge, single autosave, fair dice |
| Ironman | No reloads, harsher consequences, grudges last longer |
| Nightmare | Ironman + hostile media + frequent scandals + volatile economy + scheming NPCs |
