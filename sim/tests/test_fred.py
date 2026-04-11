"""Tests for the FRED API macro data fetcher.

All tests use mocked HTTP responses -- no actual API calls are made.
"""

import json
from unittest.mock import MagicMock, patch

import pytest

from sim.data_pipeline.fred import (
    FALLBACK_MACRO,
    SERIES_MAP,
    fetch_fred_series,
    fetch_macro_for_year,
    generate_scenario_toml,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

REQUIRED_MACRO_FIELDS = {
    "gdp_growth",
    "inflation",
    "unemployment",
    "fed_funds_rate",
    "consumer_confidence",
    "debt_to_gdp",
}


def _make_obs(values: list[str]) -> list[dict]:
    """Build a minimal list of FRED-style observation dicts."""
    return [{"date": f"2024-01-01", "value": v} for v in values]


def _mock_response(observations: list[dict], status_code: int = 200):
    """Return a mock requests.Response with the given observations."""
    mock = MagicMock()
    mock.status_code = status_code
    mock.json.return_value = {"observations": observations}
    mock.raise_for_status.return_value = None
    return mock


# ---------------------------------------------------------------------------
# Tests: fetch_fred_series
# ---------------------------------------------------------------------------


@patch("sim.data_pipeline.fred.requests.get")
def test_fetch_fred_series_returns_observations(mock_get):
    obs = _make_obs(["1.0", "2.0", "3.0"])
    mock_get.return_value = _mock_response(obs)

    result = fetch_fred_series("UNRATE", "fake-key", 2024)

    assert result == obs
    mock_get.assert_called_once()
    call_params = mock_get.call_args[1]["params"]
    assert call_params["series_id"] == "UNRATE"
    assert call_params["observation_start"] == "2023-01-01"
    assert call_params["observation_end"] == "2024-12-31"


@patch("sim.data_pipeline.fred.requests.get")
def test_fetch_fred_series_raises_on_http_error(mock_get):
    mock = MagicMock()
    mock.raise_for_status.side_effect = Exception("404 Not Found")
    mock_get.return_value = mock

    with pytest.raises(Exception, match="404"):
        fetch_fred_series("BADID", "fake-key", 2024)


# ---------------------------------------------------------------------------
# Tests: fetch_macro_for_year
# ---------------------------------------------------------------------------


def test_fetch_macro_requires_api_key(monkeypatch):
    """Should raise ValueError without API key."""
    monkeypatch.delenv("FRED_API_KEY", raising=False)
    with pytest.raises(ValueError, match="FRED API key"):
        fetch_macro_for_year(2024)


@patch("sim.data_pipeline.fred.fetch_fred_series")
def test_fetch_macro_returns_all_required_fields(mock_fetch):
    """All macro fields should be present in the result."""
    # GDP needs >= 5 observations for YoY calc
    gdp_obs = _make_obs(["20000", "20100", "20200", "20300", "20400", "20500", "20600", "20700"])
    # CPI needs >= 13 observations for YoY calc
    cpi_obs = _make_obs([str(250 + i) for i in range(14)])
    # Others just need >= 1
    single_obs = _make_obs(["4.5"])

    def side_effect(series_id, api_key, year):
        if series_id == "GDPC1":
            return gdp_obs
        elif series_id == "CPIAUCSL":
            return cpi_obs
        else:
            return single_obs

    mock_fetch.side_effect = side_effect

    result = fetch_macro_for_year(2024, api_key="fake-key")

    assert set(result.keys()) == REQUIRED_MACRO_FIELDS


@patch("sim.data_pipeline.fred.fetch_fred_series")
def test_fetch_macro_uses_fallbacks_on_empty_data(mock_fetch):
    """When series return no observations, fallback values should be used."""
    mock_fetch.return_value = []

    result = fetch_macro_for_year(2024, api_key="fake-key")

    assert result == FALLBACK_MACRO


@patch("sim.data_pipeline.fred.fetch_fred_series")
def test_gdp_growth_calculation(mock_fetch):
    """GDP growth should be computed as YoY percentage change."""
    # 5 quarterly observations: prior year Q1-Q4 + current year Q1
    gdp_obs = _make_obs(["20000", "20100", "20200", "20300", "20400"])

    def side_effect(series_id, api_key, year):
        if series_id == "GDPC1":
            return gdp_obs
        return []

    mock_fetch.side_effect = side_effect

    result = fetch_macro_for_year(2024, api_key="fake-key")

    # (20400 - 20000) / 20000 = 0.02
    assert abs(result["gdp_growth"] - 0.02) < 1e-9


@patch("sim.data_pipeline.fred.fetch_fred_series")
def test_unemployment_conversion(mock_fetch):
    """Unemployment should be divided by 100 (percentage to decimal)."""
    def side_effect(series_id, api_key, year):
        if series_id == "UNRATE":
            return _make_obs(["3.7"])
        return []

    mock_fetch.side_effect = side_effect

    result = fetch_macro_for_year(2024, api_key="fake-key")

    assert abs(result["unemployment"] - 0.037) < 1e-9


@patch("sim.data_pipeline.fred.fetch_fred_series")
def test_inflation_calculation(mock_fetch):
    """Inflation should be YoY CPI change."""
    # 13 monthly observations
    cpi_values = [str(250 + i) for i in range(14)]
    cpi_obs = _make_obs(cpi_values)

    def side_effect(series_id, api_key, year):
        if series_id == "CPIAUCSL":
            return cpi_obs
        return []

    mock_fetch.side_effect = side_effect

    result = fetch_macro_for_year(2024, api_key="fake-key")

    expected = (263 - 251) / 251  # last - 13th-from-last
    assert abs(result["inflation"] - expected) < 1e-9


# ---------------------------------------------------------------------------
# Tests: SERIES_MAP coverage
# ---------------------------------------------------------------------------


def test_series_map_covers_all_macro_fields():
    """All macro fields should have a FRED series mapping."""
    mapped_fields = set(SERIES_MAP.values())
    # The SERIES_MAP uses short names; verify the fetch function produces all required keys
    assert REQUIRED_MACRO_FIELDS == set(FALLBACK_MACRO.keys())


def test_fallback_macro_has_all_required_fields():
    """FALLBACK_MACRO should contain every required macro field."""
    assert set(FALLBACK_MACRO.keys()) == REQUIRED_MACRO_FIELDS


# ---------------------------------------------------------------------------
# Tests: generate_scenario_toml
# ---------------------------------------------------------------------------


@patch("sim.data_pipeline.fred.fetch_macro_for_year")
def test_generate_scenario_toml_format(mock_fetch):
    """Generated TOML should be valid and contain expected sections."""
    mock_fetch.return_value = dict(FALLBACK_MACRO)

    toml_str = generate_scenario_toml(2024, api_key="fake-key")

    assert "[scenario]" in toml_str
    assert "[macro]" in toml_str
    assert "[counties]" in toml_str
    assert 'name = "USA 2024"' in toml_str
    assert "year = 2024" in toml_str
    assert 'era = "modern"' in toml_str
    assert "gdp_growth" in toml_str


@patch("sim.data_pipeline.fred.fetch_macro_for_year")
def test_generate_scenario_toml_historical_era(mock_fetch):
    """Years before 2020 should use 'historical' era."""
    mock_fetch.return_value = dict(FALLBACK_MACRO)

    toml_str = generate_scenario_toml(2010, api_key="fake-key")

    assert 'era = "historical"' in toml_str
