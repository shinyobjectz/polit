"""Tests for the simplified macro economy layer."""

from __future__ import annotations

import copy
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from layers.macro_economy import MacroEconomyLayer

# ── Helpers ──────────────────────────────────────────────────────────


def _default_delta() -> dict:
    return {
        "gdp_growth_delta": 0.0,
        "inflation_delta": 0.0,
        "unemployment_delta": 0.0,
        "fed_funds_rate": 0.0,
        "consumer_confidence_delta": 0.0,
        "debt_to_gdp_delta": 0.0,
        "sector_deltas": {},
        "county_deltas": {},
        "approval_president_delta": 0.0,
        "approval_congress_delta": 0.0,
        "protest_risk_by_region": {},
        "voter_ideology_shifts": [],
        "sector_indices": {},
        "oil_price": 0.0,
        "bond_yield_10yr": 0.0,
        "foreign_power_deltas": [],
        "trade_balance_delta": 0.0,
        "migration_pressure": {},
        "corporate_actions": [],
        "narrative_seeds": [],
    }


def _equilibrium_world() -> dict:
    """World state at equilibrium — no shocks expected."""
    return {
        "macro": {
            "gdp_growth": 0.02,
            "unemployment": 0.045,
            "inflation": 0.02,
            "consumer_confidence": 100.0,
        }
    }


def _run_ticks(
    layer: MacroEconomyLayer,
    world: dict,
    events_per_tick: list[dict] | None = None,
    n: int = 1,
) -> list[dict]:
    """Run *n* ticks, returning a list of deltas (one per tick)."""
    deltas = []
    for _ in range(n):
        delta = _default_delta()
        delta = layer.step(world, events_per_tick or [], delta)
        deltas.append(delta)
    return deltas


# ── Tests ────────────────────────────────────────────────────────────


def test_no_events_equilibrium_small_drift():
    """At equilibrium with no events, deltas should be near zero."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()
    deltas = _run_ticks(layer, world, n=4)

    for d in deltas:
        assert abs(d["gdp_growth_delta"]) < 1e-6, "GDP should not move at equilibrium"
        assert abs(d["inflation_delta"]) < 1e-6, "Inflation should not move at equilibrium"
        assert abs(d["unemployment_delta"]) < 1e-6, "Unemployment should not move at equilibrium"
        assert abs(d["consumer_confidence_delta"]) < 1e-6, "Confidence should not move at equilibrium"


def test_no_events_mean_reversion():
    """Away from equilibrium with no events, variables should revert."""
    layer = MacroEconomyLayer()
    world = {
        "macro": {
            "gdp_growth": 0.04,  # above trend
            "unemployment": 0.06,  # above natural
            "inflation": 0.05,  # above target
            "consumer_confidence": 80.0,  # below 100
        }
    }
    deltas = _run_ticks(layer, world, n=1)
    d = deltas[0]

    # GDP should revert down
    assert d["gdp_growth_delta"] < 0, "GDP should revert toward trend"
    # Unemployment should revert down (toward 4.5%)
    assert d["unemployment_delta"] < 0, "Unemployment should revert toward natural rate"
    # Inflation should revert down
    assert d["inflation_delta"] < 0, "Inflation should revert toward target"
    # Confidence should revert up
    assert d["consumer_confidence_delta"] > 0, "Confidence should revert toward 100"


def test_fiscal_stimulus_spending():
    """A fiscal spending bill should boost GDP and lower unemployment."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()

    stimulus = [{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.03}]

    # Run a few ticks to let the impulse phase in
    cumulative_gdp = 0.0
    cumulative_unemp = 0.0
    cumulative_infl = 0.0
    for _ in range(13):  # ~1 quarter
        delta = _default_delta()
        # Only inject the event on tick 0
        evts = stimulus if _ == 0 else []
        delta = layer.step(world, evts, delta)
        cumulative_gdp += delta["gdp_growth_delta"]
        cumulative_unemp += delta["unemployment_delta"]
        cumulative_infl += delta["inflation_delta"]

    assert cumulative_gdp > 0, "Fiscal spending should boost GDP"
    assert cumulative_unemp < 0, "Fiscal spending should lower unemployment"
    assert cumulative_infl > 0, "Fiscal spending should nudge inflation up"


def test_rate_hike_monetary():
    """A rate hike should drag GDP and lower inflation over time."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()

    hike = [{"type": "MonetaryPolicy", "fed_funds_delta": 0.25}]

    cumulative_gdp = 0.0
    cumulative_infl = 0.0
    for i in range(13):
        delta = _default_delta()
        evts = hike if i == 0 else []
        delta = layer.step(world, evts, delta)
        cumulative_gdp += delta["gdp_growth_delta"]
        cumulative_infl += delta["inflation_delta"]

    assert cumulative_gdp < 0, "Rate hike should drag GDP"
    assert cumulative_infl < 0, "Rate hike should lower inflation"


def test_tariff_stagflation():
    """A tariff should drag GDP and push inflation up (supply shock)."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()

    tariff = [{"type": "Tariff", "rate": 0.10}]

    cumulative_gdp = 0.0
    cumulative_infl = 0.0
    for i in range(13):
        delta = _default_delta()
        evts = tariff if i == 0 else []
        delta = layer.step(world, evts, delta)
        cumulative_gdp += delta["gdp_growth_delta"]
        cumulative_infl += delta["inflation_delta"]

    assert cumulative_gdp < 0, "Tariff should drag GDP"
    assert cumulative_infl > 0, "Tariff should push inflation up"


def test_multiple_consecutive_shocks_accumulate():
    """Multiple shocks over several ticks should accumulate."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()

    # Three consecutive spending bills
    bill = [{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.02}]

    cumulative_gdp = 0.0
    for i in range(13):
        delta = _default_delta()
        evts = bill if i < 3 else []  # inject on ticks 0, 1, 2
        delta = layer.step(world, evts, delta)
        cumulative_gdp += delta["gdp_growth_delta"]

    # Compare to single shock
    layer_single = MacroEconomyLayer()
    single_gdp = 0.0
    for i in range(13):
        delta = _default_delta()
        evts = bill if i == 0 else []
        delta = layer_single.step(world, evts, delta)
        single_gdp += delta["gdp_growth_delta"]

    assert cumulative_gdp > single_gdp, "Multiple shocks should accumulate"


def test_fed_funds_taylor_rule():
    """Fed funds should follow Taylor rule."""
    layer = MacroEconomyLayer()

    # High inflation, above-trend growth
    world = {
        "macro": {
            "gdp_growth": 0.04,
            "unemployment": 0.035,
            "inflation": 0.05,
            "consumer_confidence": 100.0,
        }
    }
    delta = _default_delta()
    delta = layer.step(world, [], delta)

    # Taylor: 0.025 + 1.5*(0.05-0.02) + 0.5*(0.04-0.02) = 0.025+0.045+0.01 = 0.08
    assert abs(delta["fed_funds_rate"] - 0.08) < 0.001, (
        f"Fed funds should be ~8% but got {delta['fed_funds_rate']}"
    )


def test_fed_funds_zero_lower_bound():
    """Fed funds should not go below zero."""
    layer = MacroEconomyLayer()
    world = {
        "macro": {
            "gdp_growth": -0.05,
            "unemployment": 0.10,
            "inflation": -0.02,
            "consumer_confidence": 60.0,
        }
    }
    delta = _default_delta()
    delta = layer.step(world, [], delta)
    assert delta["fed_funds_rate"] >= 0.0, "Fed funds should not go below zero"


def test_narrative_seeds_generated():
    """Large shocks should produce narrative seeds."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()

    big_shock = [{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.10}]
    delta = _default_delta()
    delta = layer.step(world, big_shock, delta)

    # With a massive shock, at least one narrative seed should appear
    # (may take a tick for impulse to register enough)
    # Run a few more ticks
    for _ in range(5):
        delta2 = _default_delta()
        delta2 = layer.step(world, [], delta2)
        delta["narrative_seeds"].extend(delta2["narrative_seeds"])

    assert len(delta["narrative_seeds"]) > 0, "Large shocks should produce narratives"


def test_serde_tagged_event_normalised():
    """Events in serde-tagged format should be handled correctly."""
    layer = MacroEconomyLayer()
    world = _equilibrium_world()

    # Serde-tagged format: {"FiscalBill": {"spending_gdp_pct": 0.03, ...}}
    tagged = [{"FiscalBill": {"bill_type": "spending", "amount_gdp_pct": 0.03}}]

    cumulative_gdp = 0.0
    for i in range(13):
        delta = _default_delta()
        evts = tagged if i == 0 else []
        delta = layer.step(world, evts, delta)
        cumulative_gdp += delta["gdp_growth_delta"]

    assert cumulative_gdp > 0, "Serde-tagged events should be processed correctly"


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])
