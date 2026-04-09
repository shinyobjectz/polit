---
title: Geopolitical Theatre
section: 17
status: design-complete
depends_on: [01, 05, 15]
blocks: []
---

# Geopolitical Theatre

## Design Principle

You never PLAY as a foreign country. You experience geopolitics as an American politician — through briefings, votes, crises, and consequences. Access scales with office level.

## Access by Office

| Office | Geopolitical Access |
|--------|-------------------|
| Local official | Headlines only. War affects local economy. No levers. |
| State official | National Guard, trade impact on state, sister city diplomacy. |
| US House member | Vote on war auth, defense budget, sanctions. Some briefings. |
| US Senator | Treaty ratification, ambassador confirmation, committee access, classified briefings. |
| Secretary of State | Direct diplomacy, negotiations, alliance management. |
| VP / President | Everything. Nuclear codes. Troop deployment. CIA ops. Summits. |
| CIA Director | Covert operations, intelligence gathering, regime change. |
| UN Ambassador | Multilateral diplomacy, Security Council votes. |

## Foreign Power Entity

```
Nation Entity (ECS)
├─ name, government_type, leader_npc
├─ alignment:  ally | partner | neutral | rival | adversary | enemy
├─ power:      military, economic, diplomatic, nuclear (each 1-100)
├─ interests[] territorial, economic, ideological goals
├─ treaties[]  active agreements with US
├─ trade       import/export, commodities, sanctions status
├─ military    bases, force projection, nuclear capability
├─ intel       CIA assessment confidence level
└─ stability   0-100 (failed state → rock solid)
```

### Tiered Detail
- **Tier 1** (5-6 nations): Deep model, named leader NPC, active storylines. China, Russia, UK, EU, Israel, Iran.
- **Tier 2** (15-20): Moderate model, generic leader, react to major US policy.
- **Tier 3** (rest): Regional blocs. React only to major global events.

## International Events

### Crises
Military confrontation, terrorist attack, nuclear test, humanitarian disaster, hostage situation, cyberattack, allied nation under attack, embassy siege.

### Diplomatic
Summit invitation, treaty negotiation, UN Security Council vote, alliance pressure, foreign election changes, defection/asylum request.

### Economic
Oil price shock, trade war, foreign currency crisis, sanctions effects, supply chain disruption, foreign investment changes.

### Intelligence
CIA discovers foreign op, intercepted communication, mole discovered, secret weapon program, assassination plot against foreign leader.

## War System

War is a POLITICAL system, not tactical simulation. You make decisions that send troops.

### War Lifecycle

1. **Buildup**: Intel warnings, hawks vs. doves, media narrative, corporate interests, public opinion
2. **Authorization**: Congressional AUMF vote, War Powers Act (60-day clock if unilateral), UN resolution, coalition building. Your vote FOLLOWS YOU forever.
3. **Conduct**: Weekly briefings (casualties, progress, cost). Strategic decisions: surge/drawdown, rules of engagement, drone strikes, negotiation, media embedding. Domestic effects: defense spending, war economy, protests, veteran affairs, casualty-driven approval erosion. Rally-around-the-flag effect (short-term boost, rapid decay).
4. **Resolution**: Victory (surge + occupation burden), withdrawal (blame game), stalemate (worst — endless cost). Legacy persists for decades.

### War Outcome Model
- US military power vs. adversary, modified by coalition, terrain, distance, ROE, funding, public support
- Weekly progress roll with DC based on above
- Insurgency risk: conventional victory ≠ political victory
- Nuclear escalation: separate track

## Nuclear Weapons System

### The Football
As president, you have sole launch authority. No vote, no co-sign. The game WILL let you launch and WILL simulate consequences.

### Escalation Track (0-10)
| Level | State |
|-------|-------|
| 0 | Diplomatic tension |
| 3 | Military posturing |
| 5 | Conventional conflict |
| 7 | Tactical nuclear threat |
| 9 | Strategic nuclear threat |
| 10 | Launch |

Player and adversary decisions move the track. De-escalation always possible but harder at higher levels.

### Launch Consequences
- Target devastated. Retaliation probability based on capability.
- If retaliation: US cities hit, massive casualties.
- Global economic collapse. Alliance rupture or solidarity.
- Nuclear winter effects on economic/agricultural model.
- Run continues — govern the aftermath.
- Impeachment likely. Meta-progression: permanent "Used Nuclear Weapons" flag.

### If You're NOT President
Congress can't stop a launch. But: lobby president NPC, leak classified intel, propose legislation limiting first-strike authority, invoke 25th Amendment.

## Covert Operations

### Types

| Operation | Description | Risk |
|-----------|-------------|------|
| Human intelligence | Recruit foreign assets | Asset capture, diplomatic incident |
| Signals intelligence | Intercept communications | Domestic surveillance scandal |
| Cyber intelligence | Hack foreign systems | Attribution, retaliation |
| Regime change | Fund opposition, arm rebels | Blowback, exposure scandal |
| Assassination | Drone/special forces/poison | Illegal under EO 12333, exposure devastating |
| Sabotage | Destroy facilities (Stuxnet-style) | Deniability varies by method |
| Propaganda | Foreign media, social media ops | Exposure undermines credibility |
| Arms trafficking | Supply proxy forces | Weapons end up in wrong hands |

### Every Covert Op Creates an Information Entity
- Severity 6-10 depending on operation
- Exposure risk per week based on complexity
- If exposed: congressional investigation, media firestorm, possible criminal referral

### The Oversight Question
- Legal: brief the Gang of Eight
- You CAN skip (illegal, but who stops you?)
- Briefed members may leak (discretion checks)
- Un-briefed ops are higher severity if exposed

## Proxy Wars

Ongoing systems — drain resources, create divisions, cascade consequences:

- Weekly decisions: increase/decrease funding, send advisors, escalate, negotiate, withdraw
- Consequences echo: weapons in terrorist hands, refugees, veteran NPCs, commodity prices, "why are we still there?" campaign issue

## Foreign Espionage

### Economic Espionage Against US
Player response options: sanction, indict, quiet pressure, counter-espionage, do nothing (loses corporate trust)

### State Espionage (Spies in US Government)
Mole hunt arcs. If YOU are recruitment target: accept (become foreign asset — most dangerous playthrough), refuse, report.

### US Espionage Abroad
Corporate, military, political espionage. Creates info entities on both sides.
