"""Tests for the population bootstrap module."""

from __future__ import annotations

import msgpack

from sim.bootstrap.population import bootstrap_population


def test_fallback_produces_valid_counties():
    """Without API key, fallback generates valid counties."""
    counties = bootstrap_population(api_key=None, year=2022)
    assert len(counties) > 0
    for fips, county in counties.items():
        assert county.population > 0
        assert county.median_household_income > 0
        assert 0.0 <= county.unemployment_rate <= 1.0
        assert county.state != ""
        assert county.name != ""


def test_fallback_county_has_valid_distributions():
    """Fallback county household distributions are valid."""
    counties = bootstrap_population(api_key=None, year=2022)
    county = list(counties.values())[0]
    assert abs(sum(county.households.income_quintile_distribution) - 1.0) < 0.001


def test_fallback_has_all_50_states():
    """Fallback generates exactly 50 counties (one per state)."""
    counties = bootstrap_population(api_key=None, year=2022)
    assert len(counties) == 50


def test_fallback_state_filter():
    """State filter limits fallback results."""
    counties = bootstrap_population(api_key=None, year=2022, states=["CA", "NY"])
    assert len(counties) == 2
    state_names = {c.state for c in counties.values()}
    assert "California" in state_names
    assert "New York" in state_names


def test_fallback_education_distribution_sums_to_one():
    """Education distribution shares sum to ~1.0."""
    counties = bootstrap_population(api_key=None, year=2022)
    for county in counties.values():
        total = sum(county.households.education_distribution.values())
        assert abs(total - 1.0) < 0.01, f"Education distribution sums to {total}"


def test_fallback_age_distribution_sums_to_one():
    """Age distribution shares sum to ~1.0."""
    counties = bootstrap_population(api_key=None, year=2022)
    for county in counties.values():
        total = sum(county.households.age_distribution.values())
        assert abs(total - 1.0) < 0.01, f"Age distribution sums to {total}"


def test_fallback_race_distribution_sums_to_one():
    """Race distribution shares sum to ~1.0."""
    counties = bootstrap_population(api_key=None, year=2022)
    for county in counties.values():
        total = sum(county.households.race_distribution.values())
        assert abs(total - 1.0) < 0.01, f"Race distribution sums to {total}"


def test_cache_roundtrip():
    """Cached data can be loaded back."""
    counties = bootstrap_population(api_key=None, year=2022)
    # Manually cache and reload
    cache_data = {fips: c.to_dict() for fips, c in counties.items()}
    packed = msgpack.packb(cache_data)
    unpacked = msgpack.unpackb(packed, raw=False)
    assert len(unpacked) == len(counties)

    # Verify we can reconstruct County objects from unpacked data
    from sim.models.population import County

    for fips, data in unpacked.items():
        county = County.from_dict(data)
        assert county.population > 0
        assert county.state != ""
