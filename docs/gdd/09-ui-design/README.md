---
title: UI Design
section: 09
status: design-complete
depends_on: [01, 02]
blocks: [20]
---

# UI Design

## Design Philosophy

**Chat-forward interface**: The main view is a scrolling narrative stream. Everything important flows through it. The player reads down, types at the bottom, like an AI chat with a game engine behind it.

### Core Principles

1. **Chat-first** — narrative stream is home. Everything flows through it.
2. **Context-inline** — dice rolls, card plays, NPC stats appear inline in the narrative, not separate panels.
3. **Cards float in context** — playable cards appear as floating blocks near relevant moments. Not always visible.
4. **Overlays not screens** — dashboard, map, laws, deck open as floating panels over dimmed chat. Esc closes. Narrative is always home.
5. **Slash commands** — power users type `/map`, `/deck`, `/meet davis` directly. Input line is both chat and command line.
6. **Minimal chrome** — thin status bar at top. Everything else is content.

## Main Game Screen

```
┌─────────────────────────────────────────────────────────────────┐
│ POLIT │ Week 34 │ Mayor of Springfield │ AP: 6/8      [≡ Menu] │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┄┄┄┄┄┄┄┄┄┄┄┄ Week 34 Begins ┄┄┄┄┄┄┄┄┄┄┄┄                    │
│                                                                 │
│  ■ MORNING BRIEFING                                            │
│  (narrative content flows here...)                              │
│                                                                 │
│  You spent 2 AP to meet with Chief Kowalski.                   │
│                                                                 │
│  ■ CHIEF KOWALSKI                              Trust: 58       │
│  "Mayor, I'll be straight with you..."                         │
│                                                                 │
│      ┌───────────────────────────────────────┐                  │
│      │ 🎴 Play Card                          │                  │
│      │  [1] Sympathetic Ear (+2 trust, free) │                  │
│      │  [2] Budget Promise (costs 1 AP)      │                  │
│      └───────────────────────────────────────┘                  │
│                                                                 │
│ > _                                                   [AP: 4/8] │
└─────────────────────────────────────────────────────────────────┘
```

## Phase-Aware Status Bar

The status bar transforms based on current phase:

| Phase | Status Bar |
|-------|-----------|
| Free Roam | `POLIT │ Week 34 │ Mayor │ AP: 4/8` |
| Conversation | `POLIT │ Meeting: Chief Kowalski │ Trust: 58 │ Free talk` |
| Debate | `POLIT │ ⚡ DEBATE R3/5 │ Topic: Economy │ 0:42 │ You: 62pts` |
| Crisis | `POLIT │ ⚠ CRISIS: Factory Explosion │ Decisions: 2/4` |
| Floor Vote | `POLIT │ 🏛 FLOOR VOTE │ HR-1224 │ Yea:28 Nay:19` |
| Election Night | `POLIT │ 🗳 ELECTION NIGHT │ Precincts: 34%` |

## Phase-Aware Input Line

| Phase | Input Prompt |
|-------|-------------|
| Free Roam | `> What do you want to do? (/help) [AP: 4/8]` |
| Conversation | `> Speaking to Kowalski (type freely, /leave) [Free]` |
| Debate | `> Respond to moderator (/play card) [Timer: 0:42]` |
| Crisis | `> ⚠ Choose your response (2 decisions left) [Urgent]` |
| Floor Vote | `> /call <senator> to lobby, /ready to vote` |
| Draft Law | `> Write your provision (plain English) [Free]` |
| Downtime | `> Manage cards, plan ahead (/next to advance)` |
| Election Night | `> Results coming in... [Watching]` |

## Floating Command Palette

Press `Tab` or `[≡]`:

```
┌──────────────────────┐
│  ≡ COMMAND PALETTE   │
├──────────────────────┤
│  📊 Dashboard        │
│  🗺  Map              │
│  📜 Laws             │
│  🃏 Cards & Deck     │
│  👥 Relationships    │
│  📰 News Archive     │
│  💰 Finances         │
│  📋 Active Quests    │
│  👥 Staff            │
│  🔒 Intel            │
│  ⚙  Settings         │
│  💾 Save & Quit      │
│  /help for commands  │
└──────────────────────┘
```

## Overlays

Each menu item opens as a floating panel over dimmed chat. Examples:

- **Relationships**: Grouped by ally/rival/neutral, with trust bars
- **Cards & Deck**: Full collection with filters by type/rarity
- **Map**: ASCII state map with election data, demographic overlays
- **Laws**: Browse active legislation with status and effects
- **Staff**: Team management with assign/hire/fire
- **Intel**: Information tracker showing secrets you know
- **News**: Headline archive with cycle status

## Inline Phase Transitions

Phases announce themselves in the chat flow — no hard screen cuts:

```
═══════════════════════════════════════════
⚡ PRESS CONFERENCE — Channel 4 News
Topic: Water Treatment Plant Delays
You'll face 5 questions. /spin cards available.
═══════════════════════════════════════════
```

And when they end:

```
═══════════════════════════════════════════
✓ PRESS CONFERENCE COMPLETE
Performance: Strong │ Media: Mostly favorable
═══════════════════════════════════════════
```

## Slash Commands

| Command | Action |
|---------|--------|
| `/meet <npc>` | Start conversation |
| `/call <npc>` | Phone call (cheaper AP) |
| `/draft` | Enter law drafting mode |
| `/speech <topic>` | Give public speech |
| `/campaign <district>` | Campaign in a district |
| `/scheme` | Covert action options |
| `/intel` | Intelligence briefing |
| `/cards` | Open deck manager |
| `/play <card>` | Play a specific card |
| `/map` | Open map overlay |
| `/laws` | Browse active laws |
| `/stats` | Economic dashboard |
| `/graph` | Relationship network |
| `/news` | News archive |
| `/staff` | Staff management |
| `/propose <desc>` | Propose custom action to DM |
| `/inspect <event>` | See mechanics behind active event |
| `/journal` | All active custom events |
| `/end` | End turn |
| `/help [topic]` | Command reference |

## Keyboard Controls

| Key | Action |
|-----|--------|
| 1-0 | Quick-select actions/cards |
| Tab | Command palette |
| Arrow keys | Navigate lists/map |
| Enter | Confirm/select |
| Esc | Back/cancel/close overlay |
| F1 | Help overlay |
| / | Slash command mode |
| Space | End turn (with confirmation) |
| v | Toggle voice input |

## Ratatui Implementation

- Main layout: `Layout::default().constraints()` for panel splits
- Narrative: scrollable `Paragraph` with `Wrap` and styled `Spans`
- Overlays: layered rendering with dimmed background
- Input: custom widget with cursor
- Map: `Canvas` widget with custom shapes
- Views: `enum ScreenMode` — game swaps based on context
- Color scheme: configurable in `config/theme.toml`
