---
title: Tutorial & Onboarding
section: 20
status: design-complete
depends_on: [02, 09]
blocks: []
---

# Tutorial & Onboarding

## Design Principle

The game is enormous. Teach through PLAY, not walls of text. The DM IS the tutorial. First-time players get a guided scenario that naturally introduces mechanics.

## First Launch

```
┌─────────────────────────────────────────────────┐
│                                                 │
│                  P O L I T                       │
│                                                 │
│       The American Politics Simulator           │
│                                                 │
│  [N] New Campaign                               │
│  [T] Tutorial: "First Day in Office"            │
│  [S] Settings                                   │
│  [Q] Quit                                       │
│                                                 │
│  First time? Start with the tutorial. It's      │
│  a real campaign — you'll learn by doing.       │
│                                                 │
└─────────────────────────────────────────────────┘
```

## Tutorial Campaign: "First Day in Office"

Pre-built character: newly elected city council member. Small scope: one district, few NPCs, simple economy. 12-week guided run (can continue as full campaign after).

### Week 1: THE BASICS
- Morning briefing (teaches: how to read briefings)
- One meeting (teaches: conversation, typing responses)
- One action (teaches: AP system, action selection)
- DM explains the basics in narrative voice

### Week 2-3: PEOPLE
- NPC approaches you (teaches: relationships form naturally)
- Favor asked (teaches: debt system, social graph)
- Rival introduced (teaches: opposition, conflict)
- Relationship overlay tutorial tooltip

### Week 4-5: CARDS
- First card acquired (teaches: card system)
- Card play opportunity (teaches: tactics in context)
- Position card granted (teaches: coherence system)
- Deck overlay tutorial tooltip

### Week 6-7: LAWS
- Simple ordinance opportunity (teaches: drafting)
- Vote on another member's proposal (teaches: voting)
- See effects of passed law (teaches: law → simulation impact)

### Week 8-9: CRISIS
- Small crisis event (teaches: event phases)
- Dice roll (teaches: skill checks, DC system)
- Consequence plays out (teaches: permanent consequences)

### Week 10-11: NEWS & INFORMATION
- Story runs about you (teaches: news system)
- Intel about rival (teaches: information tracking)
- Decision: leak or hold (teaches: information as weapon)

### Week 12: ELECTION
- Reelection campaign begins (teaches: campaign basics)
- Short campaign arc (condensed)
- Election night
- Win or lose — both outcomes continue

### End of Tutorial

```
"You've learned the basics. There's much more to
 discover — deckbuilding strategy, economic policy,
 higher offices, covert operations, and more.
 The best way to learn is to play."

[C] Continue this campaign (full rules, no hand-holding)
[N] Start a new campaign (full character creation)
[M] Main menu
```

## Progressive Disclosure After Tutorial

Systems not covered in tutorial (staff, geopolitics, corporate lobbying, covert ops, voice input) are introduced via contextual tooltips the FIRST time the player encounters them.

- Tooltips are one-time, non-intrusive, dismissible
- `/help <system>` available anytime
- DM naturally explains new mechanics in narrative voice when they first appear

## Contextual Help System

| Command | Coverage |
|---------|----------|
| `/help` | General command reference |
| `/help cards` | Deckbuilder explanation |
| `/help economy` | Economic system overview |
| `/help elections` | How elections work |
| `/help laws` | Legislative process |
| `/help staff` | Staff management guide |
| `/help combat` | "There is no combat. This is politics." |
| `/tutorial` | Replay tutorial anytime |
