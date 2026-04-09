---
title: News & Information System
section: 15
status: design-complete
depends_on: [01, 06]
blocks: [16, 17]
---

# News & Information System

## Core Concept

Information is a RESOURCE that exists independently of whether anyone knows it. The game tracks WHAT exists, WHO knows it, and WHAT HAPPENS when it spreads.

## Information Entity

```
Information Entity (ECS)
├─ Identity
│  ├─ id, type, subject
│  ├─ type: fact | rumor | leak | spin | fabrication
│  ├─ topic: scandal | policy | personal | financial |
│  │         criminal | political_deal | affair |
│  │         health | incompetence | corruption
│  └─ about: entity ref (who/what this concerns)
├─ Content
│  ├─ truth_value      0.0-1.0 (fabrication → absolute fact)
│  ├─ severity         1-10 (parking ticket → treason)
│  ├─ newsworthiness   1-10 (boring → front page)
│  ├─ decay_rate       how fast it stops being news
│  └─ evidence_level   none | circumstantial | documented |
│                      recorded | undeniable
├─ Knowledge Graph
│  ├─ knowers[]        entities who know this
│  ├─ source_chain[]   how each knower learned it
│  ├─ public           bool — has this been published?
│  └─ public_belief    0.0-1.0 — does the public believe it?
└─ Lifecycle
   ├─ created_week
   ├─ discovered_week
   ├─ published_week (if ever)
   └─ status: secret | rumored | reported | confirmed |
              old_news | forgotten
```

## Information Lifecycle

### 1. Creation
Information created by simulation events. Only participants initially KNOW it.

### 2. Private Spread
Each week, knowers may spread based on:
- Discretion stat (low = leaky)
- Motivation (do they gain from sharing?)
- Relationship to subject (enemy = more likely)
- Roll: Discretion DC based on severity × motivation
- Follows social graph: close allies share freely, rivals share to journalists
- Each hop adds distortion if type = rumor

### 3. Media Pickup
Journalist NPC evaluates: `newsworthiness × severity = publish_score`
- Evidence must meet outlet's standards
- Hostile relationship to subject = lower threshold
- Editorial pressure from ownership may suppress or amplify
- If `publish_score > threshold` → story runs

### 4. Publication
- `info.public = true`
- Impact depends on outlet reach
- `public_belief` influenced by outlet credibility
- Approval impact calculated immediately
- News cycle begins

### 5. News Cycle
- **Breaking**: Maximum impact, dominates briefing
- **Developing**: Follow-up coverage, new angles
- **Contested**: Counter-narratives, spin wars
- **Fading**: Public attention moves on
- **Forgotten**: Minimal ongoing impact (still in record)

Duration depends on severity, competing news, player/NPC actions feeding the story, and new revelations.

## Media Ecosystem

### Media Organization Entity

```
Media Org Entity (ECS)
├─ name, type, reach (local → global)
├─ type: newspaper | tv_network | cable_news | radio |
│        digital_media | podcast | social_platform | wire_service
├─ credibility   0-100
├─ editorial     lean (-100 left to +100 right)
├─ ownership     entity ref (corporation, individual, nonprofit)
├─ priorities    coverage focus areas
├─ staff[]       journalist NPCs
└─ revenue_model ads | subscription | donor | corporate_parent
```

### Ownership Matters
- Corporate parent can suppress stories hurting their business
- Billionaire owner can push editorial direction
- Player building relationship with owner can influence coverage (risks "media manipulation" scandal)
- Independent outlets harder to influence but smaller reach

## Player Interactions

### Slash Commands

| Command | Action |
|---------|--------|
| `/news` | Current headlines + cycle status |
| `/news archive` | Past stories, searchable |
| `/news <outlet>` | Specific outlet's coverage |
| `/intel` | Private information briefing |
| `/intel <topic>` | What do you know about X? |
| `/leak <info> to <npc>` | Deliberately leak information |
| `/spin <story>` | Shape narrative (press sec helps) |
| `/suppress <story>` | Kill a story (costs AP, risky) |
| `/investigate <target>` | Dig for information |
| `/plant <story>` | Fabricate/exaggerate (high risk) |
| `/deny <story>` | Public denial (works if evidence weak) |
| `/confess <story>` | Get ahead of it (reduces damage) |

### Card Interactions
- "Media Contact" asset → +3 to suppress/spin
- "Kompromat" asset → leak devastating info about rival
- "Clean Record" position → buffs denial credibility
- "Spin Doctor" tactic → reframe any story once per week

## Cross-System Effects

| System | Interaction |
|--------|-------------|
| Social Graph | Knowing secrets = leverage. Mutual secrets = MAD (stable alliance). |
| Cards | Kompromat IS an asset card. Opposition research generates info entities. |
| Law Engine | Criminal info + documented evidence → investigation. Whistleblower/shield laws affect leaking. |
| Economy | Corporate scandal → stock impact. Corruption story → consumer confidence dip. |
| Elections | October surprise = high-severity info timed before election. Issue salience shifts with news. |
| Corporate | Corporations suppress harmful info via owned media. Plant favorable stories. |
| Staff | Competence → intel quality. Discretion → leak risk. Press sec → spin success. |
| Custom Events | Every freeform action creates info entities. Bribery → info exists, will surface eventually. |

## Morning Briefing Integration

Headlines organized by scope (national → local) with impact annotations. Intel section shows private information. Information tracker shows what you're monitoring.
