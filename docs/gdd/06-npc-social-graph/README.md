---
title: NPC System & Social Network Graph
section: 06
status: design-complete
depends_on: [01, 02]
blocks: [07, 10, 15]
---

# NPC System & Social Network Graph

## Character Entity Structure

```
Character Entity (ECS)
├─ Identity       name, age, gender, race, background
├─ Role           office held, party, faction, committees
├─ Personality    Big Five traits (OCEAN)
├─ Ideology       position on each major issue axis (0.0-1.0)
├─ Stats          persuasion, cunning, charisma, knowledge,
│                 ruthlessness, loyalty
├─ Goals          short-term + long-term (drives autonomous behavior)
├─ Memory         vector of specific events involving player
├─ Mood           current emotional state
├─ Lifecycle      health, career stage, retirement probability
└─ Sexual Orientation / Gender Identity / Family
```

## Social Network Graph (petgraph)

### Node = Character entity (player + all NPCs)

### Edge = Relationship

| Field | Range | Description |
|-------|-------|-------------|
| trust | -100 to +100 | How much they rely on you |
| respect | -100 to +100 | Professional regard |
| fear | 0 to 100 | Intimidation factor |
| loyalty | 0 to 100 | Will they stick with you |
| debt | -10 to +10 | Favors owed/owed to |
| knowledge | 0 to 100 | How well they know you |
| leverage | 0 to 100 | Secrets you hold over them |
| type | enum | ally, rival, mentor, protege, neutral, enemy, family, donor, staffer |
| memories[] | list | Specific interactions that shaped this edge |

## Reputation Propagation

Information travels through the social graph, not instantly:

1. **Direct effect**: Immediate relationship change with involved NPC
2. **First-degree propagation**: Close allies (loyalty > 60) hear about it. Attenuated impact.
3. **Second-degree propagation**: Allies of allies. Original impact × 0.3. Dampens to zero at ~3 degrees.
4. **Media amplification**: If a journalist NPC has knowledge > 50 of either party → `trigger_event("media_story")` → reputation hit goes PUBLIC, skipping graph propagation.

**Propagation timing**: NOT instant. Rumors spread 1-2 hops per week. Player can act to contain damage before it spreads.

## NPC Autonomous Behavior

Each Dawn Phase, every active NPC runs a decision loop:

1. **Evaluate goals** — Am I making progress? Has anything threatened my position?
2. **Evaluate relationships** — Who do I need? Who threatens me? Debts to call in?
3. **Choose action** (weighted random from personality + situation):
   - Propose legislation aligned with ideology
   - Form/break alliances based on changing power dynamics
   - Campaign if election approaching
   - Scheme against rivals (if ruthlessness > 60)
   - Retire if health low + career stage late
4. **Gemma 4 narrates** significant NPC actions in briefing

## NPC Lifecycle

```
GENERATION → ACTIVE → CAREER CHANGE → RETIREMENT → DEATH → REPLACEMENT
```

- **Generation**: Procedural or historical. Gemma 4 fleshes out personality.
- **Active**: Participates in politics, has goals, acts autonomously.
- **Career Change**: Wins/loses office, switches roles, joins lobby.
- **Retirement**: Leaves active politics, may still be contactable.
- **Death**: Removed from active graph, legacy effects remain.
- **Replacement**: New NPCs generated influenced by current political climate.

## Group Dynamics

Group conversations use a turn-order system:

1. Player speaks (text or audio)
2. Gemma 4 determines which NPCs react and in what order (extraverts jump in, introverts wait)
3. Each NPC response considers: relationship to player, relationship to OTHER NPCs present, goals, ideology, what's been said
4. NPCs may agree, disagree, side-eye, or redirect
5. Group mood emerges from individual reactions
6. Alliances and rivalries can shift DURING group scenes

## Staff System

Staff members are full NPC entities with additional mechanics:

### Staff Roles

| Role | Function | Key Stat |
|------|----------|----------|
| Chief of Staff | Morning briefing quality, scheduling | Competence |
| Press Secretary | Media management, spin | Media Savvy |
| Policy Advisor | Legislation drafting quality | Knowledge |
| Campaign Manager | Ground game efficiency | Cunning |
| Fundraiser | Donor cultivation | Ambition (double-edged) |
| Legal Counsel | Constitutional issue flagging | Knowledge |
| Field Organizer | District-level campaigning | Endurance |
| Scheduler | AP optimization | Competence |
| Opposition Researcher | Intel gathering | Discretion |

### Staff Dynamics
- Weekly salary from war chest
- Competence determines output quality
- Loyalty determines leak risk
- Ambition determines whether they pursue their own agenda
- High-ambition staff may: skim funds, build their own network, get poached by K Street
- Staff deployed on tasks are unavailable for the week

## Family & Personal Life

### Identity (set at creation)
- Sexual orientation, gender identity
- Marital status, children
- Religion (affects voter affinity)
- These are NOT cosmetic — they shape gameplay

### Spouse/Partner
- Full NPC entity with personality, goals, career
- Can be asset (campaign surrogate, social intel, emotional support)
- Can be liability (scandal vulnerability, political disagreement, demands time)
- Relationship health tracked — neglect = problems
- Divorce possible with era-dependent political consequences

### Children
- Age over career. May enter politics (dynasty seed for future runs).
- Parenting choices affect "family values" perception.
- Kid's school play vs. crucial vote = real dilemma.

### Personal Health & Stress
- Stress accumulates from crises, overwork, scandals
- High stress = worse dice rolls, snap decisions, health events
- Burnout possible — forced reduced AP for several weeks
- Health events scale with age + stress

### Social Identity & Voter Dynamics
- Identity affects which voter blocs identify with you
- Advantage or obstacle depending on district/era
- Gemma 4 calibrates NPC reactions to era-appropriate norms
- Player's identity is never punished by the GAME — but the simulated SOCIETY may react

## RocksDB Storage

```
Column family: "relationships"
Key:   {entity_a_id}:{entity_b_id}
Value: serialized RelationshipEdge

Column family: "npc_memories"
Key:   {entity_id}:{memory_timestamp}
Value: serialized MemoryEntry

Column family: "characters"
Key:   {entity_id}
Value: serialized Character
```

Queries via prefix iteration: all relationships for entity X = prefix scan "X:".
