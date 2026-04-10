---
title: New Game Flow & UI Redesign
section: 22
status: design-complete
depends_on: [01, 04, 08, 09]
blocks: []
---

# New Game Flow & UI Redesign

## Flow

```
polit → Loading → Main Menu → Scenario Select → Cinematic Intro → Character Creation → Game
```

## 1. Main Menu

- Dark bg `(8,8,16)`, centered flag + POLIT
- "Continue Campaign" grayed out with "(no saves)" when no saves
- Items: New Campaign, Continue Campaign, Settings, Quit

## 2. Scenario Select

- Two-step on same screen: Era → Difficulty
- Eras: Modern America, Historical, Alternate History, Speculative Future
- Difficulty: Story, Standard, Ironman, Nightmare
- "Advanced" expands inline: tone dial, accuracy, starting year
- Scenario-driven from TOML (SDK compatible)

## 3. Cinematic Intro

- Full-screen slides from `scenarios/*/intro.toml`
- Typewriter text animation, centered, dark bg
- → to advance after animation completes
- Shift+Space to skip (bottom right)
- Moddable per scenario

## 4. Character Creation (AI-Guided)

- Full-screen chat conversation with the Gemma 4 DM
- AI asks questions one at a time, locks in answers
- Locked-in details shown as inline rendered blocks:
  ```
  ┌─────────────────────────────────────────────┐
  │ ✓ Name: Alex Rivera                         │
  │ ✓ Background: Community organizer           │
  │ ✓ Archetype: The Idealist                   │
  │ ○ Traits: ...                               │
  └─────────────────────────────────────────────┘
  ```
- Progress meter top-right: `Character Depth ████░░ 62%`
  - 0-30% Basics (minimum to start)
  - 30-60% Forming
  - 60-80% Detailed
  - 80-100% Deep Lore (soft ceiling)
- Full character sheet viewable on right side (toggle)
- Player says "I'm ready" or presses → past 30% to begin
- AI uses tool calls to set up game state (cards, NPCs, vars)

## 5. In-Game UI

### Dark Theme
- Background `(8,8,16)` everywhere (matches title screen)
- Same visual language throughout all screens

### Centered Content
- Chat text has left+right margins (~10% each side)
- Content never wider than ~80 chars
- Feels like reading, not a log dump

### Minimal Status Bar (top)
- Just: week, office, AP gauge
- Thin, subtle, no branding or help text

### Clean Input (bottom)
- Just `> _` with cursor
- No phase hints or status clutter

### Slash Command Autocomplete
- Typing `/` shows filtered command menu above input
- Filters as you type (command palette style)
- Arrow to select, Enter to execute, Esc to dismiss

### NPC Avatars
- Two-char face + colored name in ratatui styled spans
- Each NPC has unique expression + color:
  ```
  °° DAVIS        ← cyan (glasses)
  ── KOWALSKI     ← yellow (stern)
  ^^ MARTINEZ     ← green (friendly)
  ¬¬ CHEN         ← red (skeptical)
  •• KIM          ← magenta (alert)
  ```

### AI-First Interaction
- Player mostly types naturally
- AI interprets intent, confirms if ambiguous
- Commands are escape hatches, not primary UX
- Multiple choice offered by AI when appropriate

## 6. View Switcher

- Tab-hold shows minimal pill bar at bottom center:
  ```
  Chat · Map · People · Deck · Laws · Econ · News
  ```
- Arrow to navigate, Enter to select
- Each view is full-screen, same dark theme
- Shortcuts bindable through Settings (not shown by default)
- Esc or selecting Chat returns to default view

## 7. Remove Mock Data

- No hardcoded starter cards, NPCs, or game state
- Everything populated through character creation + scenario files
- `--mock` flag kept for dev testing only
