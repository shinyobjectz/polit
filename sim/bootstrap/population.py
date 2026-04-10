"""County-level population bootstrap via the Census Bureau ACS API.

Entry point
-----------
``bootstrap_population(api_key, year, states) -> dict[str, County]``

Data flow:
    1. Check disk cache  (``sim/data/population_cache_{year}.msgpack``)
    2. If no cache, try the Census API (requires *api_key* or ``CENSUS_API_KEY``)
    3. If no API key, generate synthetic fallback counties (one per state)
"""

from __future__ import annotations

import logging
import os
from pathlib import Path
from typing import Any

import msgpack

from sim.models.population import County, HouseholdProfile

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Census ACS variable mapping
# ---------------------------------------------------------------------------

_ACS_VARIABLES = [
    "B01003_001E",  # Total population
    "B19013_001E",  # Median household income
    "B23025_005E",  # Unemployed (civilian labor force)
    "B23025_003E",  # Civilian labor force
    "B25002_003E",  # Vacant housing units
    "B25002_001E",  # Total housing units
    "B25003_002E",  # Owner-occupied units
    "B25003_001E",  # Occupied housing units
    # Age distribution (selected brackets from B01001)
    "B01001_003E",  # Male 5-9
    "B01001_004E",  # Male 10-14
    "B01001_005E",  # Male 15-17
    "B01001_006E",  # Male 18-19
    "B01001_007E",  # Male 20
    "B01001_008E",  # Male 21
    "B01001_009E",  # Male 22-24
    "B01001_010E",  # Male 25-29
    "B01001_011E",  # Male 30-34
    "B01001_012E",  # Male 35-39
    "B01001_013E",  # Male 40-44
    "B01001_014E",  # Male 45-49
    "B01001_015E",  # Male 50-54
    "B01001_016E",  # Male 55-59
    "B01001_017E",  # Male 60-61
    "B01001_018E",  # Male 62-64
    "B01001_019E",  # Male 65-66
    "B01001_020E",  # Male 67-69
    "B01001_021E",  # Male 70-74
    "B01001_022E",  # Male 75-79
    "B01001_023E",  # Male 80-84
    "B01001_024E",  # Male 85+
    "B01001_027E",  # Female 5-9
    "B01001_028E",  # Female 10-14
    "B01001_029E",  # Female 15-17
    "B01001_030E",  # Female 18-19
    "B01001_031E",  # Female 20
    "B01001_032E",  # Female 21
    "B01001_033E",  # Female 22-24
    "B01001_034E",  # Female 25-29
    "B01001_035E",  # Female 30-34
    "B01001_036E",  # Female 35-39
    "B01001_037E",  # Female 40-44
    "B01001_038E",  # Female 45-49
    "B01001_039E",  # Female 50-54
    "B01001_040E",  # Female 55-59
    "B01001_041E",  # Female 60-61
    "B01001_042E",  # Female 62-64
    "B01001_043E",  # Female 65-66
    "B01001_044E",  # Female 67-69
    "B01001_045E",  # Female 70-74
    "B01001_046E",  # Female 75-79
    "B01001_047E",  # Female 80-84
    "B01001_048E",  # Female 85+
    # Race distribution (B02001)
    "B02001_001E",  # Total
    "B02001_002E",  # White alone
    "B02001_003E",  # Black alone
    "B02001_004E",  # AIAN alone
    "B02001_005E",  # Asian alone
    "B02001_006E",  # NHPI alone
    # Education (B15003) — selected attainment levels
    "B15003_001E",  # Total 25+
    "B15003_002E",  # No schooling
    "B15003_003E",  # Nursery school
    "B15003_004E",  # Kindergarten
    "B15003_005E",  # 1st grade
    "B15003_006E",  # 2nd grade
    "B15003_007E",  # 3rd grade
    "B15003_008E",  # 4th grade
    "B15003_009E",  # 5th grade
    "B15003_010E",  # 6th grade
    "B15003_011E",  # 7th grade
    "B15003_012E",  # 8th grade
    "B15003_013E",  # 9th grade
    "B15003_014E",  # 10th grade
    "B15003_015E",  # 11th grade
    "B15003_016E",  # 12th grade no diploma
    "B15003_017E",  # HS diploma
    "B15003_018E",  # GED
    "B15003_019E",  # Some college < 1yr
    "B15003_020E",  # Some college 1+ yr
    "B15003_021E",  # Associate's
    "B15003_022E",  # Bachelor's
    "B15003_023E",  # Master's
    "B15003_024E",  # Professional
    "B15003_025E",  # Doctorate
]

# Cache directory relative to this file's package root
_DATA_DIR = Path(__file__).resolve().parent.parent / "data"

# ---------------------------------------------------------------------------
# Age bracket aggregation helpers
# ---------------------------------------------------------------------------

# Maps our simplified age bracket names to Census variable suffixes.
# Male variables start at B01001_003E; female at B01001_027E (same offsets +24).
_AGE_BRACKETS: dict[str, list[str]] = {
    "under_18": [
        "B01001_003E", "B01001_004E", "B01001_005E",
        "B01001_027E", "B01001_028E", "B01001_029E",
    ],
    "18_29": [
        "B01001_006E", "B01001_007E", "B01001_008E",
        "B01001_009E", "B01001_010E",
        "B01001_030E", "B01001_031E", "B01001_032E",
        "B01001_033E", "B01001_034E",
    ],
    "30_44": [
        "B01001_011E", "B01001_012E", "B01001_013E",
        "B01001_035E", "B01001_036E", "B01001_037E",
    ],
    "45_64": [
        "B01001_014E", "B01001_015E", "B01001_016E",
        "B01001_017E", "B01001_018E",
        "B01001_038E", "B01001_039E", "B01001_040E",
        "B01001_041E", "B01001_042E",
    ],
    "65_plus": [
        "B01001_019E", "B01001_020E", "B01001_021E",
        "B01001_022E", "B01001_023E", "B01001_024E",
        "B01001_043E", "B01001_044E", "B01001_045E",
        "B01001_046E", "B01001_047E", "B01001_048E",
    ],
}


def _safe_int(val: Any) -> int:
    """Convert a Census value to int, treating None / negative as 0."""
    if val is None:
        return 0
    try:
        v = int(val)
        return max(v, 0)
    except (ValueError, TypeError):
        return 0


def _safe_float(val: Any) -> float:
    """Convert a Census value to float, treating None / negative as 0.0."""
    if val is None:
        return 0.0
    try:
        v = float(val)
        return max(v, 0.0)
    except (ValueError, TypeError):
        return 0.0


def _safe_ratio(numerator: Any, denominator: Any, default: float = 0.0) -> float:
    """Compute numerator/denominator safely."""
    n = _safe_float(numerator)
    d = _safe_float(denominator)
    if d == 0.0:
        return default
    return n / d


# ---------------------------------------------------------------------------
# Row → County conversion
# ---------------------------------------------------------------------------


def _row_to_county(row: dict[str, Any], state_name: str) -> County:
    """Convert a single Census API response row into a County dataclass."""
    fips = str(row.get("state", "00")) + str(row.get("county", "000"))
    county_name = str(row.get("NAME", fips))

    total_pop = _safe_int(row.get("B01003_001E"))
    median_income = _safe_float(row.get("B19013_001E"))
    if median_income <= 0:
        median_income = 60000.0  # fallback to national average

    unemployed = _safe_int(row.get("B23025_005E"))
    labor_force = _safe_int(row.get("B23025_003E"))
    unemployment_rate = _safe_ratio(unemployed, labor_force, 0.04)

    vacant = _safe_int(row.get("B25002_003E"))
    total_housing = _safe_int(row.get("B25002_001E"))
    vacancy_rate = _safe_ratio(vacant, total_housing, 0.07)

    owner_occupied = _safe_int(row.get("B25003_002E"))
    occupied = _safe_int(row.get("B25003_001E"))
    ownership_rate = _safe_ratio(owner_occupied, occupied, 0.65)

    # --- Age distribution ---
    age_dist: dict[str, float] = {}
    for bracket, variables in _AGE_BRACKETS.items():
        age_dist[bracket] = sum(_safe_int(row.get(v)) for v in variables)
    age_total = sum(age_dist.values())
    if age_total > 0:
        age_dist = {k: v / age_total for k, v in age_dist.items()}
    else:
        age_dist = {
            "under_18": 0.22, "18_29": 0.17, "30_44": 0.20,
            "45_64": 0.25, "65_plus": 0.16,
        }

    # --- Race distribution ---
    race_total = _safe_int(row.get("B02001_001E"))
    if race_total > 0:
        white = _safe_int(row.get("B02001_002E")) / race_total
        black = _safe_int(row.get("B02001_003E")) / race_total
        aian = _safe_int(row.get("B02001_004E")) / race_total
        asian = _safe_int(row.get("B02001_005E")) / race_total
        nhpi = _safe_int(row.get("B02001_006E")) / race_total
        other = max(0.0, 1.0 - white - black - aian - asian - nhpi)
        race_dist = {
            "white": white, "black": black, "aian": aian,
            "asian": asian, "nhpi": nhpi, "other": other,
        }
    else:
        race_dist = {
            "white": 0.60, "black": 0.13, "aian": 0.01,
            "asian": 0.06, "nhpi": 0.002, "other": 0.198,
        }

    # --- Education distribution ---
    edu_total = _safe_int(row.get("B15003_001E"))
    if edu_total > 0:
        no_hs = sum(
            _safe_int(row.get(f"B15003_{i:03d}E"))
            for i in range(2, 17)  # 002-016: less than HS diploma
        ) / edu_total
        hs = sum(
            _safe_int(row.get(f"B15003_{i:03d}E"))
            for i in range(17, 19)  # 017-018: HS diploma + GED
        ) / edu_total
        some_college = sum(
            _safe_int(row.get(f"B15003_{i:03d}E"))
            for i in range(19, 22)  # 019-021: some college + associate's
        ) / edu_total
        bachelors = _safe_int(row.get("B15003_022E")) / edu_total
        graduate = sum(
            _safe_int(row.get(f"B15003_{i:03d}E"))
            for i in range(23, 26)  # 023-025: master's, professional, doctorate
        ) / edu_total
    else:
        no_hs = 0.12
        hs = 0.27
        some_college = 0.29
        bachelors = 0.20
        graduate = 0.12

    edu_dist = {
        "no_hs": no_hs, "hs": hs, "some_college": some_college,
        "bachelors": bachelors, "graduate": graduate,
    }

    households = HouseholdProfile(
        income_quintile_distribution=[0.2, 0.2, 0.2, 0.2, 0.2],
        education_distribution=edu_dist,
        age_distribution=age_dist,
        race_distribution=race_dist,
        housing_own_rent_split=ownership_rate,
    )

    return County(
        fips=fips,
        state=state_name,
        name=county_name,
        population=total_pop,
        median_household_income=median_income,
        unemployment_rate=unemployment_rate,
        housing_vacancy_rate=vacancy_rate,
        unionization_rate=0.10,
        political_lean_index=0.0,
        voter_registration={},
        turnout_propensity=0.60,
        major_industries={},
        households=households,
    )


# ---------------------------------------------------------------------------
# Census API fetch
# ---------------------------------------------------------------------------


def _fetch_from_census(
    api_key: str, year: int, states: list[str] | None,
) -> dict[str, County]:
    """Pull ACS 5-year county data from the Census API."""
    from census import Census  # type: ignore[import-untyped]
    import us as us_states  # type: ignore[import-untyped]

    c = Census(api_key, year=year)

    # Resolve state FIPS codes
    if states is not None:
        fips_list = []
        for abbr in states:
            st = us_states.states.lookup(abbr)
            if st is not None:
                fips_list.append(st.fips)
            else:
                logger.warning("Unknown state abbreviation: %s", abbr)
    else:
        fips_list = [st.fips for st in us_states.states.STATES]

    # Build FIPS → state name map
    fips_to_name: dict[str, str] = {}
    for st in us_states.states.STATES:
        fips_to_name[st.fips] = st.name

    counties: dict[str, County] = {}
    fields = tuple(_ACS_VARIABLES) + ("NAME",)

    for state_fips in fips_list:
        state_name = fips_to_name.get(state_fips, state_fips)
        logger.info("Fetching ACS data for %s (FIPS %s)…", state_name, state_fips)
        try:
            rows = c.acs5.state_county(fields, state_fips, Census.ALL)
        except Exception:
            logger.exception("Failed to fetch data for state %s", state_fips)
            continue

        if not rows:
            continue

        for row in rows:
            county = _row_to_county(row, state_name)
            counties[county.fips] = county

    logger.info("Fetched %d counties from Census API", len(counties))
    return counties


# ---------------------------------------------------------------------------
# Fallback synthetic data
# ---------------------------------------------------------------------------

# Representative default values loosely based on national averages.
_FALLBACK_STATES = [
    ("01", "Alabama", "AL"), ("02", "Alaska", "AK"), ("04", "Arizona", "AZ"),
    ("05", "Arkansas", "AR"), ("06", "California", "CA"), ("08", "Colorado", "CO"),
    ("09", "Connecticut", "CT"), ("10", "Delaware", "DE"), ("12", "Florida", "FL"),
    ("13", "Georgia", "GA"), ("15", "Hawaii", "HI"), ("16", "Idaho", "ID"),
    ("17", "Illinois", "IL"), ("18", "Indiana", "IN"), ("19", "Iowa", "IA"),
    ("20", "Kansas", "KS"), ("21", "Kentucky", "KY"), ("22", "Louisiana", "LA"),
    ("23", "Maine", "ME"), ("24", "Maryland", "MD"), ("25", "Massachusetts", "MA"),
    ("26", "Michigan", "MI"), ("27", "Minnesota", "MN"), ("28", "Mississippi", "MS"),
    ("29", "Missouri", "MO"), ("30", "Montana", "MT"), ("31", "Nebraska", "NE"),
    ("32", "Nevada", "NV"), ("33", "New Hampshire", "NH"), ("34", "New Jersey", "NJ"),
    ("35", "New Mexico", "NM"), ("36", "New York", "NY"), ("37", "North Carolina", "NC"),
    ("38", "North Dakota", "ND"), ("39", "Ohio", "OH"), ("40", "Oklahoma", "OK"),
    ("41", "Oregon", "OR"), ("42", "Pennsylvania", "PA"), ("44", "Rhode Island", "RI"),
    ("45", "South Carolina", "SC"), ("46", "South Dakota", "SD"),
    ("47", "Tennessee", "TN"), ("48", "Texas", "TX"), ("49", "Utah", "UT"),
    ("50", "Vermont", "VT"), ("51", "Virginia", "VA"), ("53", "Washington", "WA"),
    ("54", "West Virginia", "WV"), ("55", "Wisconsin", "WI"), ("56", "Wyoming", "WY"),
]


def _generate_fallback(states: list[str] | None = None) -> dict[str, County]:
    """Generate one synthetic county per state with plausible defaults."""
    selected = _FALLBACK_STATES
    if states is not None:
        upper = {s.upper() for s in states}
        selected = [s for s in _FALLBACK_STATES if s[2] in upper]

    counties: dict[str, County] = {}
    for state_fips, state_name, _abbr in selected:
        fips = state_fips + "001"
        households = HouseholdProfile(
            income_quintile_distribution=[0.2, 0.2, 0.2, 0.2, 0.2],
            education_distribution={
                "no_hs": 0.12, "hs": 0.27, "some_college": 0.29,
                "bachelors": 0.20, "graduate": 0.12,
            },
            age_distribution={
                "under_18": 0.22, "18_29": 0.17, "30_44": 0.20,
                "45_64": 0.25, "65_plus": 0.16,
            },
            race_distribution={
                "white": 0.60, "black": 0.13, "aian": 0.01,
                "asian": 0.06, "nhpi": 0.002, "other": 0.198,
            },
            housing_own_rent_split=0.65,
        )
        county = County(
            fips=fips,
            state=state_name,
            name=f"{state_name} County",
            population=200_000,
            median_household_income=60_000.0,
            unemployment_rate=0.04,
            housing_vacancy_rate=0.07,
            unionization_rate=0.10,
            political_lean_index=0.0,
            voter_registration={},
            turnout_propensity=0.60,
            major_industries={},
            households=households,
        )
        counties[fips] = county

    return counties


# ---------------------------------------------------------------------------
# Cache helpers
# ---------------------------------------------------------------------------


def _cache_path(year: int) -> Path:
    return _DATA_DIR / f"population_cache_{year}.msgpack"


def _load_cache(year: int) -> dict[str, County] | None:
    path = _cache_path(year)
    if not path.exists():
        return None
    try:
        with open(path, "rb") as f:
            raw = msgpack.unpackb(f.read(), raw=False)
        return {fips: County.from_dict(d) for fips, d in raw.items()}
    except Exception:
        logger.exception("Failed to load cache from %s", path)
        return None


def _save_cache(counties: dict[str, County], year: int) -> None:
    _DATA_DIR.mkdir(parents=True, exist_ok=True)
    path = _cache_path(year)
    data = {fips: c.to_dict() for fips, c in counties.items()}
    with open(path, "wb") as f:
        f.write(msgpack.packb(data))
    logger.info("Saved %d counties to cache %s", len(counties), path)


# ---------------------------------------------------------------------------
# Public entry point
# ---------------------------------------------------------------------------


def bootstrap_population(
    api_key: str | None = None,
    year: int = 2022,
    states: list[str] | None = None,
) -> dict[str, County]:
    """Bootstrap county population data.

    Parameters
    ----------
    api_key:
        Census Bureau API key.  Falls back to ``CENSUS_API_KEY`` env var,
        then to synthetic default data.
    year:
        ACS 5-year survey year (default 2022).
    states:
        Optional list of state abbreviations to restrict to.
        ``None`` means all 50 states.

    Returns
    -------
    dict mapping FIPS code (str) to :class:`County`.
    """
    # 1. Check disk cache
    cached = _load_cache(year)
    if cached is not None:
        logger.info("Loaded %d counties from cache", len(cached))
        if states is not None:
            upper = {s.upper() for s in states}
            # Filter cached counties by requested states
            import us as us_states  # type: ignore[import-untyped]
            abbr_to_name = {st.abbr: st.name for st in us_states.states.STATES}
            names = {abbr_to_name.get(a, a) for a in upper}
            cached = {f: c for f, c in cached.items() if c.state in names}
        return cached

    # 2. Resolve API key
    key = api_key or os.environ.get("CENSUS_API_KEY")

    if key:
        # 3a. Fetch from Census API
        counties = _fetch_from_census(key, year, states)
        if counties:
            _save_cache(counties, year)
            return counties
        logger.warning("Census API returned no data; falling back to synthetic")

    # 3b. Fallback
    logger.info("No Census API key configured; using synthetic fallback data")
    return _generate_fallback(states)
