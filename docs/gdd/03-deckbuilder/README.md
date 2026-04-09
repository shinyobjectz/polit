---
title: Deckbuilder System
section: 03
status: design-complete
depends_on: [01, 02]
blocks: [08, 10]
---

# Deckbuilder System

## Card Taxonomy

Three card types that interact:

### Tactic Cards (what you DO)

Actions you can take. Each has AP cost, requirements, and effects.

| Category | Examples |
|----------|---------|
| Political | Filibuster, Logroll, Whip Votes |
| Media | Press Conference, Leak, Op-Ed, Interview |
| Campaign | Rally, Ad Blitz, Door-to-Door, Debate Prep |
| Covert | Opposition Research, Backroom Deal, Bribe |
| Legal | File Lawsuit, Subpoena, Executive Order |

### Asset Cards (what you HAVE)

Resources and relationships. Persistent — stay in play until lost.

| Category | Examples |
|----------|---------|
| People | Ally, Mentor, Informant, Fixer, Donor |
| Organizations | PAC, Union, Corporation, Media Org |
| Resources | War Chest, Polling Data, Kompromat |
| Institutional | Committee Seat, Chairmanship, Staff Position |

### Position Cards (what you BELIEVE)

Your public identity. Create synergies and vulnerabilities.

| Category | Examples |
|----------|---------|
| Economic | Free Trade, Protectionist, Austerity |
| Social | Progressive, Traditional, Libertarian |
| Foreign Policy | Hawk, Dove, Isolationist |
| Governance | Big Government, Small Government |
| Wedge Issues | Specific hot-button stances |

## Card Interactions — The Combo System

- **Tactic cards have requirements**: "Filibuster" requires an institutional asset (committee seat) and costs AP. "Leak to Press" requires a media org asset.
- **Asset cards are persistent**: They stay in play until lost (ally betrays you, org withdraws support, resource spent).
- **Position cards create synergies and contradictions**:
  - Aligned positions buff each other: "Free Trade" + "Pro-Business" = +2 to corporate fundraising
  - Contradictory positions create vulnerability: "Pro-Environment" + "Pro-Drilling" = flip-flop flag
  - Gemma 4 tracks position coherence; NPCs and media call out contradictions

## The Flip-Flop Mechanic

```
Coherence Score = aligned_pairs - contradictory_pairs

High coherence (>5):  "Principled" — base trusts you, media respects
                       you, but you're predictable and rigid
Low coherence (<-3):  "Flip-flopper" — flexibility but trust erodes,
                       opponents run attack ads, base enthusiasm drops
Neutral (0 to 5):     "Pragmatist" — balanced, no bonuses or penalties
```

## Card Rarity and Progression

| Rarity | How Acquired | Examples |
|--------|-------------|---------|
| Common | Starting deck, routine events | Stump Speech, Local Donor, Moderate Stance |
| Uncommon | Quest rewards, relationship milestones | Media Contact, Committee Seat, Policy Wonk |
| Rare | Major achievements, risky plays | Corporate Mega-Donor, Kingmaker Ally, Landmark Position |
| Legendary | Once-per-run opportunities, critical successes | Supreme Court Nomination, Constitutional Amendment, Party Realignment |

## Card Evolution

- **Upgrade through use**: Play "Stump Speech" 10 times → evolves into "Master Orator" with better stats
- **Degradation if neglected**: Don't maintain relationships and allies drift away
- **Position deepening**: Doubling down on a position strengthens it but narrows your coalition

## Deck Construction

- Start each run with a small starter deck based on character background
- Cards acquired through gameplay (win negotiation = ally card, pass bill = position card)
- **Max deck size** (representing bandwidth/attention) — adding past the limit forces discards
- You can't stand for everything and know everyone

## Meta-Progression Cards

Completing runs unlocks new **archetypes** (starter deck templates) and adds discovered cards to the permanent card pool. Unlocked cards appear in future runs but must still be earned in-game.

See [Character & Meta-Progression](../08-character-meta-progression/README.md) for full details.
