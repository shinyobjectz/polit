---
title: Economic & World Simulation
section: 05
status: design-complete
depends_on: [01, 12]
blocks: [07, 15, 16]
---

# Economic & World Simulation

## Layered Architecture

### Layer 1: Surface Indicators (casual players)

Simple gauges anyone can read at a glance:

- Economy: Strong/Moderate/Weak
- Unemployment: percentage
- Approval on economy: percentage
- Deficit: dollar amount
- Stability: gauge
- Inequality: trend arrow

### Layer 2: Macro Model (expert players)

Full economic variables running underneath:

- GDP (real & nominal)
- CPI / Inflation
- Labor force participation
- Housing starts
- Sectoral output (agriculture, manufacturing, tech, finance, energy)
- State-level breakdowns
- Tax revenue by source
- Federal funds rate
- Trade balance by sector
- Debt-to-GDP ratio
- Consumer confidence
- Gini coefficient
- Government spending by department

### Layer 3: Demographic Model

Population dynamics per district:

- Age, race, income, education distribution
- Migration patterns (internal + immigration)
- Urban/rural shifts
- Birth/death rates
- Voter registration and turnout propensity

### Layer 4: Geopolitical Model

See [Geopolitics](../17-geopolitics/README.md) for full detail:

- Major power relationships
- Trade agreements and sanctions
- Military deployments and conflicts
- Global commodity prices

## Tick Pipeline (runs every Dawn Phase)

1. **Exogenous shocks** — random events from event tables (oil crisis, pandemic, tech boom). Scheduled historical events in historical mode. Gemma 4 can inject narrative-driven shocks.

2. **Policy effects propagate** — active laws apply modifiers to economic variables. Tax changes → revenue + growth effects (lagged). Spending → sector + employment effects. Regulatory changes → industry + compliance costs.

3. **Economic model steps** — supply/demand equilibrium per sector, interest rate responds to inflation/employment, trade balance adjusts to tariffs + global prices, budget computes.

4. **Demographic model steps** — population ages/migrates based on economic pull, voter sentiment shifts based on personal economics, issue salience recalculates.

5. **Geopolitical model steps** — foreign powers react to US policy, trade relationships evolve, conflict/diplomacy events may trigger.

6. **Aggregate to surface indicators** — compute Layer 1 display values from Layer 2-4 data.

## Policy → Outcome Causality Chains

Laws don't just change a number — they trigger multi-step causal chains with realistic lag:

**Example: Minimum wage increase ($15 → $20)**

| Week | Effect |
|------|--------|
| 1 | Law enacted. No immediate economic effect. |
| 2-4 | Small business compliance costs rise (+2%). Worker income rises. Consumer spending ticks up in low-income districts. |
| 5-8 | Some small business closures in marginal areas. Unemployment ticks up slightly. Inflation nudges (0.1-0.3%). Approval shifts: +5 labor, -8 business. |
| 9+ | Second-order effects stabilize. Net depends on economic conditions at time of passage. |

These chains are defined in **policy effect templates** (TOML/Rhai) — moddable, tunable.

## Game Start Modes

| Mode | Data Source | Description |
|------|------------|-------------|
| Modern | Latest API snapshot | Real current economy |
| Historical | Historical FRED/Census data | Economy at any past year |
| Alternate History | Fork from historical point | Diverge from real conditions |
| Speculative | Trend projection + parameters | Extrapolated future |
| Fictional | Scenario TOML files | Fully custom |

## Data Sources

| Source | Data | Purpose |
|--------|------|---------|
| FRED API | 840k+ time series | GDP, unemployment, inflation, rates |
| Census Bureau | District demographics | Population, income, education by district |
| BLS | Employment by sector/region | Labor market model |
| BEA | GDP by state/county | Regional variation |
| World Bank | Global indicators | Geopolitical model baseline |

See [Data Pipeline](../12-data-pipeline/README.md) for ingestion details.
