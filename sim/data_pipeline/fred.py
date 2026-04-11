"""FRED API macro data fetcher for scenario generation.

Fetches macroeconomic indicators from the Federal Reserve Economic Data (FRED)
API and returns them in the format expected by our scenario system.
"""

import os
from datetime import datetime

import requests

FRED_BASE = "https://api.stlouisfed.org/fred/series/observations"

SERIES_MAP = {
    "GDPC1": "gdp",
    "UNRATE": "unemployment",
    "CPIAUCSL": "cpi",
    "FEDFUNDS": "fed_funds_rate",
    "UMCSENT": "consumer_confidence",
    "GFDEGDQ188S": "debt_to_gdp",
}

FALLBACK_MACRO = {
    "gdp_growth": 0.02,
    "inflation": 0.025,
    "unemployment": 0.04,
    "fed_funds_rate": 0.05,
    "consumer_confidence": 100.0,
    "debt_to_gdp": 1.2,
}


def fetch_fred_series(series_id: str, api_key: str, year: int) -> list[dict]:
    """Fetch a single FRED series for a given year.

    Requests observations from the start of the prior year through the end
    of the target year so that year-over-year calculations are possible.
    """
    params = {
        "series_id": series_id,
        "api_key": api_key,
        "file_type": "json",
        "observation_start": f"{year - 1}-01-01",
        "observation_end": f"{year}-12-31",
    }
    resp = requests.get(FRED_BASE, params=params, timeout=30)
    resp.raise_for_status()
    return resp.json().get("observations", [])


def fetch_macro_for_year(year: int, api_key: str | None = None) -> dict:
    """Fetch all macro indicators for a year, return scenario-compatible dict.

    Returns a dict matching the [macro] section of scenario TOML::

        {gdp_growth, inflation, unemployment, fed_funds_rate,
         consumer_confidence, debt_to_gdp}

    Falls back to ``FALLBACK_MACRO`` defaults for any series that cannot be
    computed from the returned observations.
    """
    api_key = api_key or os.environ.get("FRED_API_KEY")
    if not api_key:
        raise ValueError(
            "FRED API key required. Set FRED_API_KEY env var or pass api_key."
        )

    macro = dict(FALLBACK_MACRO)  # start with defaults

    # GDP growth: compute YoY % change from quarterly real GDP
    gdp_obs = fetch_fred_series("GDPC1", api_key, year)
    if len(gdp_obs) >= 5:  # need prior year for YoY
        current = float(gdp_obs[-1]["value"])
        prior = float(gdp_obs[-5]["value"])  # 4 quarters back
        macro["gdp_growth"] = (current - prior) / prior

    # Unemployment: latest value, convert from percentage
    unemp_obs = fetch_fred_series("UNRATE", api_key, year)
    if unemp_obs:
        macro["unemployment"] = float(unemp_obs[-1]["value"]) / 100

    # Inflation: YoY CPI change
    cpi_obs = fetch_fred_series("CPIAUCSL", api_key, year)
    if len(cpi_obs) >= 13:
        current = float(cpi_obs[-1]["value"])
        prior = float(cpi_obs[-13]["value"])  # 12 months back
        macro["inflation"] = (current - prior) / prior

    # Fed funds rate
    ff_obs = fetch_fred_series("FEDFUNDS", api_key, year)
    if ff_obs:
        macro["fed_funds_rate"] = float(ff_obs[-1]["value"]) / 100

    # Consumer confidence/sentiment
    sent_obs = fetch_fred_series("UMCSENT", api_key, year)
    if sent_obs:
        macro["consumer_confidence"] = float(sent_obs[-1]["value"])

    # Debt to GDP
    debt_obs = fetch_fred_series("GFDEGDQ188S", api_key, year)
    if debt_obs:
        macro["debt_to_gdp"] = float(debt_obs[-1]["value"]) / 100

    return macro


def generate_scenario_toml(year: int, api_key: str | None = None) -> str:
    """Fetch real data and generate a complete scenario TOML string."""
    macro = fetch_macro_for_year(year, api_key)

    lines = [
        "[scenario]",
        f'name = "USA {year}"',
        f'description = "Real economic data from FRED for {year}"',
        f'era = "{"modern" if year >= 2020 else "historical"}"',
        f"year = {year}",
        "",
        "[macro]",
    ]
    for key, val in macro.items():
        lines.append(f"{key} = {val}")

    lines.extend(
        [
            "",
            "[counties]",
            'source = "fetch"',
            "states = []",
        ]
    )

    return "\n".join(lines)
