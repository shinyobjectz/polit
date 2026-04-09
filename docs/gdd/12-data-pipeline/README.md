---
title: Data Pipeline & Real-World Seeding
section: 12
status: design-complete
depends_on: []
blocks: [05, 16, 17, 18]
---

# Data Pipeline & Real-World Seeding

## Architecture

Offline pipeline — runs before game, caches locally:

```
API Fetch → Normalize & Clean → Validate & Cross-reference → Store in data/ (TOML)
```

## Data Sources

### FRED (Federal Reserve Economic Data)
- API: `api.stlouisfed.org` (free key)
- Series: GDP, GDPC1, UNRATE, CPIAUCSL, FEDFUNDS, GFDEBTN, HOUST, UMCSENT, GINI, M2SL, DTWEXBGS
- Historical depth: 1940s-present
- Update: quarterly fetch, cached locally

### Census Bureau
- API: `api.census.gov` (free key)
- Datasets: ACS 5-year (demographics by district), Decennial (full counts), Population estimates
- Fields: age, race, income, education, occupation, housing, language, veteran status
- Granularity: state → county → tract → block group → congressional districts

### BLS (Bureau of Labor Statistics)
- API: `api.bls.gov` (free, rate-limited)
- Series: employment by sector, wages by occupation, CPI components, productivity

### BEA (Bureau of Economic Analysis)
- API: `apps.bea.gov` (free key)
- Tables: GDP by state, personal income by county, industry output, trade balance

### Wikipedia / Wikidata
- MediaWiki API: `en.wikipedia.org/w/api.php` (free)
- Wikidata SPARQL: `query.wikidata.org` (free)
- Uses: historical event timelines, politician bios, government structure, legal precedent, geographic context

### Additional Sources

| Source | Data | Purpose |
|--------|------|---------|
| USAspending.gov | Federal spending by agency/program | Department budget seeding |
| Congress.gov | Bill text, voting records, members | Legislative data |
| OpenSecrets | Campaign finance, lobbying spend | Corporate political activity |
| SEC EDGAR | Public company filings | Corporate entity seeding |
| FBI UCR | Crime statistics by jurisdiction | Law enforcement context |
| CDC WONDER | Public health data | Health system context |
| OPM FedScope | Federal workforce data | Government department seeding |
| GS Pay Scale | Federal salary tables | Bureaucratic career seeding |
| Federal Register | Regulations by agency | Regulatory context |
| World Bank | Global economic indicators | Geopolitical model |

## Seeding by Game Mode

### Modern (2024+)
- Economy: latest FRED snapshot
- Demographics: latest ACS 5-year
- Politicians: Wikidata query for current officeholders
- Laws: Congress.gov for major active legislation
- Geopolitics: cached summary of current world affairs

### Historical (year selector)
- Economy: FRED historical series for target year
- Demographics: closest census to target year
- Politicians: Wikidata for officeholders at that time
- Events: Wikipedia timeline loaded for upcoming years — events WILL happen unless player changes history
- Accuracy modes: STRICT (on schedule), MODERATE (seeded but changeable), LOOSE (starting conditions only)

### Alternate History
- Same as Historical up to fork point
- Player defines divergence ("What if JFK survived?")
- Gemma 4 extrapolates immediate consequences
- Historical timeline AFTER fork is discarded

### Speculative
- Extrapolate from latest data using trend projection
- Player parameters: years forward, climate scenario, tech trajectory, political trajectory
- Gemma 4 generates world state from parameters

### Fictional
- No real data used. Entirely defined by scenario TOML files.

## Historical Fact-Checking System

When accuracy mode is STRICT or MODERATE:

1. Pre-cache relevant Wikipedia articles for the era (~500-2000 articles)
2. DM tool: `check_historical(claim)` → confirmed/unconfirmed
3. Player can ask "did this really happen?" → sourced answer with confidence
4. Historical drift tracking:
   - ✓ happened as in reality
   - ✗ prevented by player actions
   - ~ altered outcome
   - ★ new event (no historical parallel)
5. End-of-run "historical divergence score"

## Data CLI

| Command | Purpose |
|---------|---------|
| `polit-data fetch --all` | Refresh everything |
| `polit-data fetch --source fred` | Just economic data |
| `polit-data fetch --historical 1960-1970` | Specific era |
| `polit-data status` | Show cache freshness |
| `polit-data export --scenario modern_usa` | Bake into scenario files |
| `polit-data validate` | Cross-reference integrity |
