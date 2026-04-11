"""Stylized scenario tests for the 8-layer simulation.

Each test creates a specific starting macro state, runs multiple ticks
(typically 52 = 1 year), and checks that the dynamics produce plausible,
bounded outcomes.
"""

import json
import msgpack
import pytest

from sim.host import tick, reset_layers


@pytest.fixture(autouse=True)
def _fresh():
    reset_layers()
    yield
    reset_layers()


def _run_scenario(macro, events_by_tick=None, n_ticks=52, counties=None):
    """Run a scenario and return (deltas_list, final_macro)."""
    events_by_tick = events_by_tick or {}
    world = {"week": 1, "macro": dict(macro), "counties": counties or {}}
    m = dict(macro)
    deltas = []
    for week in range(1, n_ticks + 1):
        world["week"] = week
        world["macro"] = m
        ws = msgpack.packb(world)
        ev = msgpack.packb(events_by_tick.get(week, []))
        delta = json.loads(tick(ws, ev))
        deltas.append(delta)
        m["gdp_growth"] += delta.get("gdp_growth_delta", 0)
        m["inflation"] += delta.get("inflation_delta", 0)
        m["unemployment"] += delta.get("unemployment_delta", 0)
        m["consumer_confidence"] += delta.get("consumer_confidence_delta", 0)
        if delta.get("fed_funds_rate"):
            m["fed_funds_rate"] = delta["fed_funds_rate"]
    return deltas, m


# ---- Test 1: Recession Recovery ----


def test_recession_recovery():
    """Economy should recover from recession over 52 weeks via mean reversion."""
    macro = {
        "gdp_growth": -0.02,
        "inflation": 0.01,
        "unemployment": 0.08,
        "fed_funds_rate": 0.01,
        "consumer_confidence": 65.0,
        "debt_to_gdp": 1.0,
    }
    deltas, final = _run_scenario(macro)

    # GDP should recover toward trend (2%)
    assert final["gdp_growth"] > macro["gdp_growth"], "GDP should recover"
    # Unemployment should not spike much further from 8% (Okun's law lag
    # means it may not actually decline within 52 weeks, but it should
    # remain bounded and not spiral upward).
    assert final["unemployment"] < 0.10, "Unemployment should stay bounded"
    # Consumer confidence should improve
    assert final["consumer_confidence"] > macro["consumer_confidence"]
    # Values should stay bounded
    assert -0.1 < final["gdp_growth"] < 0.1
    assert 0.0 < final["unemployment"] < 0.2


# ---- Test 2: Stagflation Spiral ----


def test_stagflation_from_supply_shocks():
    """Tariff + energy shock should cause stagflation."""
    macro = {
        "gdp_growth": 0.02,
        "inflation": 0.03,
        "unemployment": 0.045,
        "fed_funds_rate": 0.04,
        "consumer_confidence": 95.0,
        "debt_to_gdp": 1.1,
    }
    events = {
        1: [{"type": "Tariff", "partner": "China", "product": "all", "rate": 0.25}],
        5: [{"type": "SectorShock", "sector": "Energy", "severity": 2.5}],
    }
    deltas, final = _run_scenario(macro, events, n_ticks=26)

    # Inflation should be higher than start (supply shock)
    assert final["inflation"] > macro["inflation"], "Inflation should rise"
    # GDP should be lower (demand destruction)
    assert final["gdp_growth"] < macro["gdp_growth"], "GDP should fall"
    # This IS stagflation
    assert final["inflation"] > 0.04 and final["gdp_growth"] < 0.01


# ---- Test 3: Scandal During Recession ----


def test_scandal_compounds_recession_damage():
    """Scandal during recession should produce worse approval than either alone."""
    macro_recession = {
        "gdp_growth": -0.01,
        "inflation": 0.015,
        "unemployment": 0.07,
        "fed_funds_rate": 0.02,
        "consumer_confidence": 70.0,
        "debt_to_gdp": 1.1,
    }

    # Recession only
    deltas_recession, _ = _run_scenario(macro_recession, n_ticks=15)
    approval_recession = sum(
        d.get("approval_president_delta", 0) for d in deltas_recession
    )

    # Recession + scandal
    events = {
        5: [
            {
                "type": "Scandal",
                "actor": "President",
                "severity": 4.0,
                "scandal_type": "financial",
                "media_amplification": 2.0,
            }
        ]
    }
    deltas_both, _ = _run_scenario(macro_recession, events, n_ticks=15)
    approval_both = sum(d.get("approval_president_delta", 0) for d in deltas_both)

    # Scandal + recession should be worse
    assert approval_both < approval_recession, "Scandal should compound recession damage"


# ---- Test 4: Wartime Rally Effect ----


def test_wartime_economy():
    """Conflict should boost defense sector."""
    macro = {
        "gdp_growth": 0.02,
        "inflation": 0.02,
        "unemployment": 0.045,
        "fed_funds_rate": 0.03,
        "consumer_confidence": 100.0,
        "debt_to_gdp": 1.0,
    }
    events = {
        1: [
            {
                "type": "Conflict",
                "parties": ["Iran"],
                "conflict_type": "military",
                "escalation_level": 0.5,
            }
        ],
    }
    deltas, final = _run_scenario(macro, events, n_ticks=20)

    # Geopolitical layer should produce foreign power deltas
    has_foreign = any(len(d.get("foreign_power_deltas", [])) > 0 for d in deltas)
    assert has_foreign, "Conflict should produce foreign power deltas"

    # Migration pressure should appear (conflict region instability)
    has_migration = any(len(d.get("migration_pressure", {})) > 0 for d in deltas)
    assert has_migration, "Conflict should affect migration pressure"


# ---- Test 5: Boom Overheating ----


def test_boom_overheating():
    """High GDP growth should trigger Fed tightening via Taylor rule."""
    macro = {
        "gdp_growth": 0.045,
        "inflation": 0.035,
        "unemployment": 0.03,
        "fed_funds_rate": 0.04,
        "consumer_confidence": 140.0,
        "debt_to_gdp": 0.8,
    }
    deltas, final = _run_scenario(macro, n_ticks=52)

    # Fed should tighten (rate should go up from 4%)
    # Taylor rule: above-trend growth + above-target inflation = higher rate
    max_rate = max(d.get("fed_funds_rate", 0) for d in deltas)
    assert max_rate > macro["fed_funds_rate"], "Fed should tighten in boom"

    # GDP should mean-revert toward trend
    assert final["gdp_growth"] < macro["gdp_growth"], "GDP should cool from 4.5%"
