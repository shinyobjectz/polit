"""County and household population data models.

These schemas define what a "county" looks like in the simulation —
demographics, economics, and political leanings. All simulation layers
read from and update these county records.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any


@dataclass
class HouseholdProfile:
    """Statistical household profile for a county (not individual households)."""

    income_quintile_distribution: list[float] = field(
        default_factory=lambda: [0.2] * 5
    )  # 5 quintiles summing to 1.0
    education_distribution: dict[str, float] = field(
        default_factory=dict
    )  # no_hs, hs, some_college, bachelors, graduate → share
    age_distribution: dict[str, float] = field(
        default_factory=dict
    )  # age brackets → share
    race_distribution: dict[str, float] = field(
        default_factory=dict
    )  # race/ethnicity → share
    housing_own_rent_split: float = 0.65  # ownership rate
    food_insecurity_rate: float = 0.10
    insurance_coverage_rate: float = 0.90

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a plain dict suitable for msgpack."""
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> HouseholdProfile:
        """Deserialize from a plain dict (e.g. after msgpack unpack)."""
        return cls(**data)


@dataclass
class County:
    """County-level population and economic data."""

    fips: str = ""
    state: str = ""
    name: str = ""
    population: int = 0
    area_sq_miles: float = 0.0
    median_household_income: float = 60000.0
    unemployment_rate: float = 0.04
    major_industries: dict[str, float] = field(
        default_factory=dict
    )  # sector → employment share
    housing_vacancy_rate: float = 0.07
    unionization_rate: float = 0.10
    political_lean_index: float = 0.0  # -1 left, +1 right
    urban_rural: str = "suburban"  # urban, suburban, exurban, rural
    voter_registration: dict[str, float] = field(
        default_factory=dict
    )  # dem, rep, ind → share
    turnout_propensity: float = 0.60
    households: HouseholdProfile = field(default_factory=HouseholdProfile)

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a plain dict suitable for msgpack."""
        d = asdict(self)
        return d

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> County:
        """Deserialize from a plain dict (e.g. after msgpack unpack)."""
        raw = dict(data)
        if "households" in raw and isinstance(raw["households"], dict):
            raw["households"] = HouseholdProfile.from_dict(raw["households"])
        return cls(**raw)
