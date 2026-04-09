---
title: Law Engine & Constitutional System
section: 07
status: design-complete
depends_on: [01, 05, 06]
blocks: [18, 19]
---

# Law Engine & Constitutional System

## Law Lifecycle

```
DRAFT → INTRODUCE → COMMITTEE → FLOOR DEBATE → VOTE →
(if bicameral) OTHER CHAMBER → EXECUTIVE SIGN/VETO →
ENACTED → ENFORCED → (possibly) CHALLENGED → COURT RULING
```

## Law Entity Structure

```
Law Entity (ECS)
├─ Identity
│  ├─ id, title, short_name
│  ├─ jurisdiction (federal / state / local)
│  ├─ type (statute / executive_order / regulation /
│  │        amendment / ordinance / resolution)
│  └─ sponsor (character entity ref)
├─ Content
│  ├─ player_draft     raw text as player wrote it
│  ├─ legal_text       Gemma 4 converts to legal language
│  ├─ plain_summary    human-readable summary
│  └─ provisions[]     structured list of discrete provisions
├─ Mechanical Effects
│  ├─ economic_modifiers[]    changes to sim variables
│  ├─ card_effects[]          grant/revoke/modify cards
│  ├─ roll_modifiers[]        change DCs or dice bonuses
│  ├─ demographic_effects[]   impact on populations
│  └─ npc_reaction_rules[]    how factions respond
├─ Status
│  ├─ stage (draft|committee|floor|enacted|struck_down)
│  ├─ votes_for / votes_against / abstentions
│  ├─ amendments[]
│  └─ legal_challenges[]
└─ Metadata
   ├─ enacted_week, expires_week (sunset clause)
   ├─ constitutional_basis
   └─ precedent_refs[]
```

## The Drafting Process

1. Player types intent in plain English
2. Gemma 4 (using `legal_style.toml` prompt) converts to legal language
3. AI identifies mechanical effects (economic modifiers, card effects)
4. AI flags constitutional issues and challenge likelihood
5. Returns structured Law entity for player review
6. Player can accept, revise, or request redraft

## Federal Legislative Process

1. **Drafting** — player writes intent, AI formalizes
2. **Sponsorship** — find co-sponsors (relationship checks)
3. **Introduction** — assigned to committee
4. **Committee** — hearings, markups, amendments. Roll: Policy Knowledge DC based on complexity + opposition. Most bills die here.
5. **Floor Rules** — Rules Committee sets debate terms
6. **Floor Debate** — special event phase (card play + speech + negotiation)
7. **Vote** — each NPC votes based on: ideology alignment, party pressure, relationship to sponsor, constituent pressure, deals made (logrolling)
8. **Conference** — reconcile House/Senate versions (if bicameral)
9. **Executive** — president signs, vetoes (override = 2/3), or pocket vetoes
10. **Enforcement** — law enters active simulation
11. **Challenge** — NPCs or events may trigger court case
12. **Court Ruling** — upheld / struck down / modified

Similar pipelines defined for: state legislatures (simplified), local ordinances, executive orders, regulatory rulemaking, constitutional amendments (Article V), judicial appointments.

## Enforcement System — Making Laws Matter

### Type 1: Mechanical Enforcement (automatic)
Laws with `economic_modifiers` apply directly to simulation variables. Min wage = $20 means the model uses $20. No ambiguity.

### Type 2: Card-Based Enforcement (triggered)
Laws grant/modify/restrict cards. "Campaign finance reform" → "Corporate Mega-Donor" card becomes illegal to play. If played anyway → scandal event.

### Type 3: Roll-Based Enforcement (probabilistic)
Some laws create DCs for certain actions. "Anti-corruption act" → bribery tactic now requires Deception check DC 18 (was DC 10). Failure = investigation.

### Type 4: RAG-Based Enforcement (AI-interpreted)
Complex laws stored in full text. When player or NPC takes an action, Gemma 4 RAG-queries active laws to determine legality. Returns: legal/illegal/ambiguous + reasoning. Ambiguous → may trigger court case.

### Type 5: Constitutional Supremacy
The constitution (base document + all amendments) is the supreme law. ALL other laws checked against it. Laws that violate can be challenged. Court rulings interpret it. Amendments modify it. Player as president can try to stretch interpretation (roll check + political consequences).

**Enforcement priority**: 5 > 1 > 2 > 3 > 4

## The Constitution as a Living Document

```
game/scenarios/modern_usa/
├─ constitution.toml          full text + structured provisions
├─ amendments/
│  ├─ 01_bill_of_rights.toml
│  └─ ...
└─ interpretations/           court rulings that modify meaning
   ├─ citizens_united.toml
   └─ ...
```

Each provision has:
- Original text
- Current interpretation (updated by in-game court rulings)
- Mechanical effects
- Challenge history

Player-passed amendments MODIFY this live document. Court rulings UPDATE interpretations. The constitution evolves through gameplay.

Scenario mods can provide entirely different constitutional frameworks (parliamentary, corporate charter, etc.). The law engine is constitution-agnostic.
