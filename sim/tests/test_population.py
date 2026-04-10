"""Tests for County and HouseholdProfile population data models."""

import math

import msgpack

from sim.models.population import County, HouseholdProfile


def test_household_income_quintiles_sum_to_one():
    """Default HouseholdProfile income quintile distribution sums to 1.0."""
    hp = HouseholdProfile()
    assert len(hp.income_quintile_distribution) == 5
    assert math.isclose(sum(hp.income_quintile_distribution), 1.0)


def test_household_roundtrip_msgpack():
    """HouseholdProfile survives msgpack serialization roundtrip."""
    hp = HouseholdProfile(
        education_distribution={"hs": 0.3, "bachelors": 0.25, "graduate": 0.15},
        age_distribution={"18-24": 0.12, "25-44": 0.35, "45-64": 0.30, "65+": 0.23},
    )
    packed = msgpack.packb(hp.to_dict())
    unpacked = msgpack.unpackb(packed)
    restored = HouseholdProfile.from_dict(unpacked)
    assert restored == hp


def test_county_roundtrip_msgpack():
    """County survives a full to_dict → packb → unpackb → from_dict cycle."""
    county = County(
        fips="39049",
        state="OH",
        name="Franklin County",
        population=1323807,
        area_sq_miles=543.5,
        median_household_income=62000.0,
        unemployment_rate=0.04,
        major_industries={"healthcare": 0.18, "education": 0.12, "finance": 0.10},
        housing_vacancy_rate=0.06,
        unionization_rate=0.09,
        political_lean_index=-0.05,
        urban_rural="urban",
        voter_registration={"dem": 0.42, "rep": 0.35, "ind": 0.23},
        turnout_propensity=0.63,
        households=HouseholdProfile(
            income_quintile_distribution=[0.18, 0.20, 0.22, 0.20, 0.20],
            education_distribution={
                "no_hs": 0.08,
                "hs": 0.25,
                "some_college": 0.20,
                "bachelors": 0.28,
                "graduate": 0.19,
            },
            age_distribution={"18-24": 0.14, "25-44": 0.32, "45-64": 0.28, "65+": 0.26},
            race_distribution={"white": 0.62, "black": 0.23, "hispanic": 0.07, "asian": 0.06, "other": 0.02},
            housing_own_rent_split=0.55,
            food_insecurity_rate=0.12,
            insurance_coverage_rate=0.92,
        ),
    )
    packed = msgpack.packb(county.to_dict())
    unpacked = msgpack.unpackb(packed)
    restored = County.from_dict(unpacked)
    assert restored == county


def test_county_franklin_oh_realistic():
    """County with real-ish data for Franklin County, OH."""
    county = County(
        fips="39049",
        state="OH",
        name="Franklin County",
        population=1323807,
        median_household_income=62000.0,
        unemployment_rate=0.04,
    )
    assert county.fips == "39049"
    assert county.state == "OH"
    assert county.population > 1_000_000
    assert 50_000 < county.median_household_income < 80_000
    assert 0.0 < county.unemployment_rate < 0.10
