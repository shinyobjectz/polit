---
title: Corporate System
section: 16
status: design-complete
depends_on: [05, 12, 15]
blocks: []
---

# Corporate System

## Design Philosophy

Corporations exist as POLITICAL FORCES, not business simulations. We don't model their P&L. We model their POLITICAL BEHAVIOR as a function of their interests.

## Data-Driven Generation

- **Public companies**: Seeded from SEC EDGAR / BLS data (name, sector, employee count, HQ, revenue tier, lobbying spend)
- **Private companies**: Generated from Census Bureau County Business Patterns (aggregated as sector blocs, e.g., "Springfield manufacturing sector")
- **Foreign subsidiaries**: Flagged with parent country
- Mapped to districts by employee/facility distribution
- Auto-generated interest profiles from sector lookup tables

## Corporate Entity (Simplified)

```
Corporate Entity (ECS)
├─ name            real or generated
├─ type            public | private_aggregate | foreign_sub
├─ sector          mapped to NAICS codes
├─ size_tier       small | mid | large | mega
├─ districts[]     where they have employees (political weight)
├─ interests[]     auto-derived from sector lookup table
├─ lobbyist        NPC entity (per mega corp, shared per sector for smaller)
├─ pac_budget      campaign donation capacity
├─ foreign_ties    parent company country (if foreign sub)
└─ leverage        f(employees_in_district, revenue, sector)
```

## Sector Interest Lookup Table

| Sector | Wants | Opposes | Lobby Intensity | Donation Pattern |
|--------|-------|---------|----------------|-----------------|
| Energy | Deregulation, drilling rights, tax breaks | Carbon tax, renewable mandates | High | Mostly right |
| Tech | H1B immigration, IP protection, Section 230 | Data privacy regulation, content liability | Very high | Mixed lean left |
| Pharma | Patent extension, fast FDA, no price controls | Drug price negotiation, generics | Very high | Bipartisan |
| Defense | Military spending, foreign intervention | Defense cuts, diplomacy first | Very high | Bipartisan hawk |
| Finance | Deregulation, low capital requirements | Dodd-Frank, transaction tax | Very high | Bipartisan establishment |
| Manufacturing | Tariffs, infrastructure, tax incentives | Min wage increase, environmental regs | Moderate | Mostly right |
| Agriculture | Subsidies, water rights, trade deals | Environmental regs, labor protections | Moderate | Mostly right |
| Healthcare | Reimbursement rates, liability caps | Single payer, price transparency | High | Bipartisan |
| Retail | Low min wage, reduced benefits mandates | Union protections, scheduling laws | Moderate | Mostly right |

## Action/Reaction Matrix

When player proposes/passes legislation:

```
impact = calculate_policy_impact(law, corp.interests)
reaction_intensity = impact × corp.lobby_intensity
```

| Impact | Corporate Reaction |
|--------|-------------------|
| +3 or more | Major donation, public endorsement |
| +1 to +2 | Quiet donation, favorable coverage if media-owned |
| 0 | No reaction |
| -1 to -2 | Lobbyist requests meeting, donations shift to opponent |
| -3 to -5 | Active opposition: attack ads, lobbying allies to block |
| -5 or less | NUCLEAR: threaten plant closure, fund primary challenger, legal challenge |

## Corporate Actions (autonomous, Dawn Phase)

| Action | Mechanics |
|--------|-----------|
| `lobby(target, issue, budget)` | Request meeting, offer donation or threaten withdrawal |
| `donate(target, amount, vehicle)` | Direct (limited), PAC (unlimited), dark money (untraceable) |
| `retaliate(policy, method)` | Relocate jobs, fund opponent, media campaign, legal challenge |
| `invest(district, jobs, conditions)` | Create jobs in exchange for favorable policy |

## Lobbying Gameplay

Lobbyist NPCs actively seek meetings. They offer campaign donations, endorsements, intel, job offers. They want favorable legislation, regulatory relief, contracts, tax breaks. Relationship with lobbyists is an asset but a liability if exposed.

## Campaign Finance

- Corporate PAC donations (limited by campaign finance law)
- Super PAC spending (independent, unlimited, less controllable)
- Dark money through nonprofits (hard to trace, creates info entities if exposed)
- Player can accept or reject — consequences either way
- Campaign finance reform changes these mechanics mid-game

## Foreign Influence

### Legal Channels
- Foreign-owned US subsidiaries lobby normally
- Sovereign wealth fund investment leverage
- FARA-registered lobbying firms
- Cultural/educational soft power

### Illegal Channels (create info entities if discovered)
- Straw donations through US nationals
- Undisclosed foreign agent activity (FARA violation)
- Cyber operations, social media manipulation
- Blackmail via foreign intelligence (kompromat)
- Accepting foreign money knowingly = high-severity info entity, criminal liability

## Data Sources

| Source | Data |
|--------|------|
| SEC EDGAR | Public company filings, lobbying disclosure |
| OpenSecrets.org | Lobbying spend, PAC donations, revolving door |
| Census CBP | Private sector: establishments + employees by sector by county |
| BEA | Industry output by region |
| BLS | Employment by industry by area |
