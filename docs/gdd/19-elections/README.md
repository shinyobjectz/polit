---
title: Election System
section: 19
status: design-complete
depends_on: [02, 03, 06, 07, 15]
blocks: [20]
---

# Election System

## Election Types

| Type | Description |
|------|-------------|
| Primary | Party nomination contest |
| General | Two+ party contest |
| Special | Filling vacancy |
| Runoff | No majority in first round |
| Recall | Voter-initiated removal |
| Ballot initiative | Direct democracy measure |
| Party convention | Delegate-based nomination |

Each scales to office level: local (personal contact), state (media buys), federal (party machinery), presidential (Electoral College).

## Election Arc Phases

### 1. Exploratory (4-8 weeks before)
- Decide whether to run (`/run for <office>`)
- Assess viability: money, name recognition, base support
- Recruit campaign staff (manager, fundraiser, field director, comms, pollster)
- Announce candidacy (speech phase — sets the tone)
- Other NPCs announce — opponents emerge
- Media coverage: favorable or skeptical based on reputation

### 2. Primary Campaign (8-16 weeks)
- Campaign within your party for nomination
- Debate other party candidates (debate phase)
- Seek endorsements from party figures
- Fundraise (war chest determines ad capacity)
- Position card management: primary voters want ideological purity (strong positions = primary advantage, may hurt in general)
- District-by-district campaigning via campaign map overlay
- Primary election day — votes tallied by district

### 3. General Campaign (8-16 weeks)
- Pivot concerns: primary positions may alienate general electorate (flip-flop risk)
- Opposition research: info entities surface about both candidates
- Debates with other party's candidate
- Ad campaigns: allocate war chest to ad buys
  - Positive ads (boost approval)
  - Attack ads (reduce opponent's, risk backlash)
  - Issue ads (shift salience in your favor)
- October surprise potential (info system at max tension)
- Final push: last week gives AP bonuses for campaign

### 4. Election Day
No more actions. Results stream in:

```
District 1 (Urban):    YOU 62% ████████░░  OPP 38%
District 2 (Suburban): YOU 51% ██████░░░░  OPP 49%  ← CLOSE
District 3 (Rural):    OPP 58% ████████░░  YOU 42%
...
Counting... ░░░░░░░░░░░░░░░░ 42% precincts
```

### 5. Aftermath
- Win: transition period, staff hiring, agenda setting
- Lose: career pivot — try again? Switch office? Become lobbyist?
- Lose gracefully: legacy points, NPC respect preserved

## Vote Calculation

Per district:
```
base = voter_registration × turnout_propensity
```

Modified by:
- Ideology match (your positions vs. district median)
- Approval rating in district
- Campaign investment (AP + money spent)
- Name recognition
- Endorsements (weighted by endorser's local influence)
- Economic conditions (incumbents punished in recession)
- News cycle (recent coverage positive/negative)
- Turnout effects (enthusiasm gap, weather, suppression)
- Debate performance modifier
- Opponent strength
- Dice roll (small random factor — elections are noisy)

## Presidential Election

### Primary Gauntlet
- Iowa caucuses, New Hampshire primary, Super Tuesday
- Delegate allocation (proportional or winner-take-all by state)
- Momentum mechanic: early wins boost name recognition and fundraising
- Delegate math: need majority to clinch
- Contested convention if no majority (rare, dramatic)
- VP selection: pick running mate NPC with complementary strengths

### Electoral College
- 538 electors, need 270 to win
- State-by-state (winner-take-all in most)
- Swing state identification from simulation's demographic/political data
- Campaign map overlay for resource allocation

### Edge Cases
- Electoral tie (269-269) → House decides
- Faithless electors (constitutional crisis)
- Contested results (recount, legal challenges)
- Third-party spoiler (NPC independent splits vote)

## Campaign Map Overlay

```
┌─────────────────────────────────────────────────┐
│ Electoral Map │ You: 214 │ Opp: 191 │ Toss: 133│
│                                                  │
│  Swing States:                                   │
│  Pennsylvania (19) ██████░░░░ Lean Opp    $2.1M │
│  Michigan (15)     ███████░░░ Toss-up     $1.8M │
│  Wisconsin (10)    ███████░░░ Toss-up     $1.2M │
│  Arizona (11)      ██████░░░░ Lean Opp    $900K │
│  Georgia (16)      ████████░░ Lean You    $1.5M │
│                                                  │
│  Budget: $4.2M │ Weeks left: 6                  │
│  [+][-] Allocate  [R]ally  [A]d buy             │
└─────────────────────────────────────────────────┘
```

## Python Simulation: Election Inputs

### compute_election_inputs()

Vote calculation formula inputs are now grounded in simulation reality via `compute_election_inputs()`, called on-demand when Rust requests election data (not a per-tick layer).

### ElectionInputs

| Field | Source Layer | Description |
|-------|-------------|-------------|
| `ideology_distribution` | Political | Per-district left/center/right breakdown |
| `turnout_propensity` | Political + Household | Base turnout adjusted by enthusiasm and income |
| `economic_conditions` | Macro + Household | GDP growth, unemployment, real income changes |
| `approval_ratings` | Political | Incumbent/candidate approval per district |
| `issue_salience` | Political + Media | Which issues voters care about, media-amplified |
| `swing_counties` | Political | Counties with narrow partisan margins |
| `enthusiasm_gap` | Political | Differential mobilization between parties |

### Integration

The vote calculation formula itself is unchanged — ideology match, approval, campaign investment, endorsements, economic conditions, news cycle, turnout, debate performance, opponent strength, and dice roll all still apply. The difference is that these inputs now come from the running simulation rather than static data or simple formulas.
