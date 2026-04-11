import math
import time

import msgpack
import pytest
from sim.host import tick, reset_layers


@pytest.fixture(autouse=True)
def _fresh():
    reset_layers()
    yield
    reset_layers()


def _run(n_ticks, events_by_tick=None, counties=None):
    """Run n ticks through the full pipeline, accumulating macro state."""
    events_by_tick = events_by_tick or {}
    macro = {
        "gdp_growth": 0.02, "inflation": 0.02, "unemployment": 0.045,
        "fed_funds_rate": 0.025, "consumer_confidence": 100.0, "debt_to_gdp": 1.0,
    }
    world = {"week": 1, "macro": macro, "counties": counties or {}}
    deltas = []
    for week in range(1, n_ticks + 1):
        world["week"] = week
        ws = msgpack.packb(world)
        ev = msgpack.packb(events_by_tick.get(week, []))
        delta = msgpack.unpackb(tick(ws, ev), raw=False)
        deltas.append(delta)
        # Accumulate
        macro["gdp_growth"] += delta.get("gdp_growth_delta", 0)
        macro["inflation"] += delta.get("inflation_delta", 0)
        macro["unemployment"] += delta.get("unemployment_delta", 0)
        macro["consumer_confidence"] += delta.get("consumer_confidence_delta", 0)
        if delta.get("fed_funds_rate"):
            macro["fed_funds_rate"] = delta["fed_funds_rate"]
    return deltas, macro


def test_52_week_run_no_events():
    """A full year with no events should remain bounded and stable."""
    deltas, macro = _run(52)
    assert -0.1 < macro["gdp_growth"] < 0.1, f"GDP out of bounds: {macro['gdp_growth']}"
    assert 0.0 < macro["unemployment"] < 0.2, f"Unemployment out of bounds: {macro['unemployment']}"
    assert -0.05 < macro["inflation"] < 0.3, f"Inflation out of bounds: {macro['inflation']}"
    assert macro["consumer_confidence"] > 50, f"Confidence too low: {macro['consumer_confidence']}"
    # No NaN or Inf
    for d in deltas:
        for key, val in d.items():
            if isinstance(val, float):
                assert val == val, f"NaN in {key}"  # NaN != NaN
                assert abs(val) < float('inf'), f"Inf in {key}"


def test_52_week_run_with_events():
    """A full year with mixed events should stay bounded."""
    events = {
        5: [{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.02}],
        15: [{"type": "Tariff", "partner": "China", "product": "electronics", "rate": 0.15}],
        25: [{"type": "Scandal", "actor": "Senator", "severity": 2.0, "scandal_type": "financial", "media_amplification": 1.0}],
        35: [{"type": "SectorShock", "sector": "Energy", "region": "Gulf", "severity": 1.5}],
        45: [{"type": "Sanction", "target_country": "Russia", "intensity": 0.5}],
    }
    deltas, macro = _run(52, events)
    # Should stay in plausible bounds even under stress
    assert -0.2 < macro["gdp_growth"] < 0.2
    assert 0.0 < macro["unemployment"] < 0.3
    assert macro["consumer_confidence"] > 0


def test_all_delta_fields_populated():
    """After events, all WorldStateDelta fields should be populated."""
    events = {
        1: [
            {"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.02},
            {"type": "SectorShock", "sector": "Energy", "severity": 1.0},
        ],
    }
    deltas, _ = _run(4, events)
    last = deltas[-1]

    # Macro fields
    assert "gdp_growth_delta" in last
    assert "inflation_delta" in last
    assert "unemployment_delta" in last
    assert "fed_funds_rate" in last
    assert "consumer_confidence_delta" in last

    # Sector fields
    assert len(last.get("sector_deltas", {})) > 0
    assert len(last.get("sector_indices", {})) > 0

    # Political fields
    assert "approval_president_delta" in last
    assert "approval_congress_delta" in last

    # Market fields
    assert "oil_price" in last
    assert "bond_yield_10yr" in last

    # Geopolitical fields
    assert "foreign_power_deltas" in last
    assert "migration_pressure" in last

    # Corporate fields
    assert "corporate_actions" in last

    # DM hooks
    assert "narrative_seeds" in last
    assert len(last["narrative_seeds"]) > 0


def test_no_nan_or_inf_in_any_field():
    """Stress test: no NaN or Inf values anywhere in any tick."""
    events = {
        1: [{"type": "EconomyShock", "magnitude": 5.0}],
        5: [{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.05}],
        10: [{"type": "Tariff", "partner": "China", "product": "all", "rate": 0.50}],
    }
    deltas, _ = _run(20, events)

    def check_value(val, path=""):
        if isinstance(val, float):
            assert not math.isnan(val), f"NaN at {path}"
            assert not math.isinf(val), f"Inf at {path}"
        elif isinstance(val, dict):
            for k, v in val.items():
                check_value(v, f"{path}.{k}")
        elif isinstance(val, list):
            for i, v in enumerate(val):
                check_value(v, f"{path}[{i}]")

    for i, d in enumerate(deltas):
        check_value(d, f"tick_{i}")


def test_narrative_seeds_generated_throughout():
    """Over 20 ticks with events, narrative seeds should be generated."""
    events = {
        1: [{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.03}],
        10: [{"type": "Scandal", "actor": "Governor", "severity": 4.0, "scandal_type": "personal", "media_amplification": 2.0}],
    }
    deltas, _ = _run(20, events)
    all_seeds = []
    for d in deltas:
        all_seeds.extend(d.get("narrative_seeds", []))
    assert len(all_seeds) >= 3, f"Expected narrative seeds, got {len(all_seeds)}"


def test_tick_performance():
    """Single tick should complete in under 500ms."""
    reset_layers()
    ws = msgpack.packb({
        "week": 1,
        "macro": {"gdp_growth": 0.02, "inflation": 0.02, "unemployment": 0.045,
                  "fed_funds_rate": 0.025, "consumer_confidence": 100.0, "debt_to_gdp": 1.0},
        "counties": {},
    })
    ev = msgpack.packb([{"type": "FiscalBill", "bill_type": "spending", "amount_gdp_pct": 0.02}])

    # Warm up
    tick(ws, ev)
    reset_layers()

    # Measure
    start = time.perf_counter()
    for _ in range(10):
        tick(ws, ev)
    elapsed = (time.perf_counter() - start) / 10

    assert elapsed < 0.5, f"Tick took {elapsed:.3f}s, target is <500ms"
    print(f"Average tick: {elapsed*1000:.1f}ms")
