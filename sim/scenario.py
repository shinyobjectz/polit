"""Scenario TOML schema and loader.

Loads scenario TOML files and produces the world_state dict
that the simulation host expects.
"""

from __future__ import annotations

import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


_DEFAULT_MACRO: dict[str, float] = {
    "gdp_growth": 0.02,
    "inflation": 0.02,
    "unemployment": 0.045,
    "fed_funds_rate": 0.025,
    "consumer_confidence": 100.0,
    "debt_to_gdp": 1.0,
}

_VALID_ERAS = {"modern", "historical", "alternate", "speculative", "fictional"}


@dataclass
class ScenarioConfig:
    """Parsed scenario configuration."""

    name: str = "Default"
    description: str = ""
    era: str = "modern"
    year: int = 2024

    # Macro starting conditions
    macro: dict[str, float] = field(default_factory=lambda: dict(_DEFAULT_MACRO))

    # County config
    county_source: str = "fallback"
    county_states: list[str] = field(default_factory=list)

    # Geopolitical overrides  {country_name: {field: value}}
    geopolitical_overrides: dict[str, dict[str, Any]] = field(default_factory=dict)

    # Scheduled events  {week: [event_dict, ...]}
    scheduled_events: dict[int, list[dict[str, Any]]] = field(default_factory=dict)


def load_scenario(path: str | Path) -> ScenarioConfig:
    """Load a scenario from a TOML file.

    Raises
    ------
    FileNotFoundError
        If *path* does not exist.
    tomllib.TOMLDecodeError
        If the file is not valid TOML.
    ValueError
        If required fields have invalid values.
    """
    path = Path(path)
    with open(path, "rb") as f:
        data = tomllib.load(f)

    config = ScenarioConfig()

    # [scenario] section
    scenario = data.get("scenario", {})
    config.name = scenario.get("name", config.name)
    config.description = scenario.get("description", config.description)
    config.era = scenario.get("era", config.era)
    config.year = scenario.get("year", config.year)

    if config.era not in _VALID_ERAS:
        raise ValueError(
            f"Invalid era '{config.era}'. Must be one of: {', '.join(sorted(_VALID_ERAS))}"
        )

    # [macro] section — merge with defaults so callers always get a full set
    if "macro" in data:
        config.macro.update(data["macro"])

    # [counties] section
    counties = data.get("counties", {})
    config.county_source = counties.get("source", config.county_source)
    config.county_states = counties.get("states", config.county_states)

    # [geopolitical.overrides]
    config.geopolitical_overrides = data.get("geopolitical", {}).get("overrides", {})

    # [[events.scheduled]]
    events = data.get("events", {})
    for event in events.get("scheduled", []):
        # Copy so we don't mutate the parsed TOML dict
        event = dict(event)
        week = event.pop("week", 1)
        config.scheduled_events.setdefault(week, []).append(event)

    return config


def scenario_to_world_state(config: ScenarioConfig) -> dict[str, Any]:
    """Convert a ScenarioConfig to the world_state dict the sim host expects."""
    return {
        "week": 1,
        "macro": dict(config.macro),
        "counties": {},  # populated by bootstrap or fetch
    }


# Equilibrium starting conditions — use when no scenario file is provided.
DEFAULT_SCENARIO = ScenarioConfig()
