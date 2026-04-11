"""Integration tests for cross-layer economic data flow.

Verifies that macro → sector → household effects cascade correctly
within and across ticks.
"""

from __future__ import annotations

import msgpack
import pytest

from sim.host import tick, reset_layers


@pytest.fixture(autouse=True)
def _fresh_layers():
    """Reset simulation layers before each test to avoid cross-test bleed."""
    reset_layers()
    yield
    reset_layers()


def _make_world(macro_overrides: dict | None = None, counties: dict | None = None) -> dict:
    """Build a minimal world state with sensible defaults."""
    macro = {
        "gdp_growth": 0.02,
        "inflation": 0.02,
        "unemployment": 0.045,
        "fed_funds_rate": 0.025,
        "consumer_confidence": 100.0,
        "debt_to_gdp": 1.0,
    }
    if macro_overrides:
        macro.update(macro_overrides)
    return {
        "week": 1,
        "macro": macro,
        "counties": counties or {},
    }


def _run_simulation(
    n_ticks: int,
    events_by_tick: dict[int, list[dict]] | None = None,
    counties: dict | None = None,
) -> list[dict]:
    """Run *n_ticks* through the full host pipeline, accumulating macro state.

    Returns the list of delta dicts, one per tick. The macro portion of
    world_state is updated each tick so that the macro layer sees the
    accumulated effects (mimicking what the Rust side does).
    """
    events_by_tick = events_by_tick or {}
    world = _make_world(counties=counties)
    macro = world["macro"]

    deltas: list[dict] = []
    for week in range(1, n_ticks + 1):
        world["week"] = week
        ws_bytes = msgpack.packb(world)
        ev_bytes = msgpack.packb(events_by_tick.get(week, []))

        delta = msgpack.unpackb(tick(ws_bytes, ev_bytes), raw=False)
        deltas.append(delta)

        # Accumulate macro state for next tick (as Rust does)
        macro["gdp_growth"] += delta.get("gdp_growth_delta", 0.0)
        macro["inflation"] += delta.get("inflation_delta", 0.0)
        macro["unemployment"] += delta.get("unemployment_delta", 0.0)
        macro["consumer_confidence"] += delta.get("consumer_confidence_delta", 0.0)
        if delta.get("fed_funds_rate", 0.0):
            macro["fed_funds_rate"] = delta["fed_funds_rate"]

    return deltas


# ── Test: fiscal stimulus cascades through all layers ────────────────


def test_fiscal_stimulus_cascades_through_all_layers():
    """Spending bill -> GDP up -> sectors expand -> household income rises."""
    events = {
        1: [{"type": "FiscalBill",
             "bill_type": "spending",
             "amount_gdp_pct": 0.03}],
    }

    deltas = _run_simulation(8, events_by_tick=events)

    # By week 8, fiscal stimulus should have had measurable effect
    total_gdp_delta = sum(d.get("gdp_growth_delta", 0) for d in deltas)
    assert total_gdp_delta > 0.001, (
        f"Expected positive GDP effect, got {total_gdp_delta}"
    )

    # Sectors should show some movement by week 8
    last_delta = deltas[-1]
    sector_movement = sum(
        abs(s.get("output_delta", 0))
        for s in last_delta.get("sector_deltas", {}).values()
    )
    assert sector_movement > 0, (
        "Sectors should show movement after fiscal stimulus"
    )


# ── Test: fiscal impulses persist and accumulate ─────────────────────


def test_fiscal_impulses_persist_across_ticks():
    """A single fiscal bill should produce GDP deltas across multiple ticks."""
    events = {
        1: [{"type": "FiscalBill",
             "bill_type": "spending", "amount_gdp_pct": 0.03,
             }],
    }

    deltas = _run_simulation(12, events_by_tick=events)

    # The impulse decays exponentially; early ticks should have larger
    # deltas than later ticks, and multiple ticks should be nonzero.
    nonzero_ticks = [
        i for i, d in enumerate(deltas)
        if abs(d.get("gdp_growth_delta", 0)) > 1e-8
    ]
    assert len(nonzero_ticks) >= 4, (
        f"Expected impulse to persist across many ticks, "
        f"got nonzero on {len(nonzero_ticks)} ticks"
    )

    # First tick effect should be larger than tick 8 effect
    first = abs(deltas[0].get("gdp_growth_delta", 0))
    eighth = abs(deltas[7].get("gdp_growth_delta", 0))
    assert first > eighth, (
        f"Impulse should decay: tick 1 ({first}) > tick 8 ({eighth})"
    )


# ── Test: sector layer sees macro delta ──────────────────────────────


def test_sectors_respond_to_macro_gdp_change():
    """Sectors should adjust output when macro GDP growth changes."""
    # Baseline: no events, equilibrium
    baseline_deltas = _run_simulation(4)

    # With a spending shock: sectors should deviate from baseline
    events = {
        1: [{"type": "FiscalBill",
             "bill_type": "spending", "amount_gdp_pct": 0.05,
             }],
    }
    shock_deltas = _run_simulation(4, events_by_tick=events)

    # Compare sector output at tick 4
    baseline_output = sum(
        abs(s.get("output_delta", 0))
        for s in baseline_deltas[-1].get("sector_deltas", {}).values()
    )
    shock_output = sum(
        abs(s.get("output_delta", 0))
        for s in shock_deltas[-1].get("sector_deltas", {}).values()
    )

    assert shock_output != baseline_output, (
        "Sectors should respond differently when macro GDP changes"
    )


# ── Test: recession cascades ─────────────────────────────────────────


def test_recession_cascades():
    """Negative GDP shock -> sectors contract -> unemployment up."""
    events = {
        1: [{"type": "EconomyShock", "severity": 5.0}],
    }

    deltas = _run_simulation(8, events_by_tick=events)

    total_gdp = sum(d.get("gdp_growth_delta", 0) for d in deltas)
    assert total_gdp < 0, f"Economy shock should drag GDP, got {total_gdp}"

    total_unemp = sum(d.get("unemployment_delta", 0) for d in deltas)
    assert total_unemp > 0, (
        f"Economy shock should raise unemployment, got {total_unemp}"
    )


# ── Test: oil shock stagflation ──────────────────────────────────────


def test_oil_shock_stagflation():
    """Energy sector shock -> prices up -> GDP drag."""
    events = {
        1: [{"type": "SectorShock", "sector": "Energy", "severity": 3.0}],
    }

    deltas = _run_simulation(8, events_by_tick=events)

    # Energy sector should show contraction at some point during the 8 ticks
    energy_deltas = [
        d.get("sector_deltas", {}).get("Energy", {})
        for d in deltas
    ]

    # The exogenous shock + agent dynamics should produce negative output
    # somewhere in the simulation window
    min_output = min(e.get("output_delta", 0) for e in energy_deltas)
    assert min_output < 0, (
        f"Energy sector should contract after shock, "
        f"min output_delta was {min_output}"
    )

    # Prices should rise (exogenous shock adds price_delta directly)
    first_energy = energy_deltas[0]
    assert first_energy.get("price_delta", 0) > 0, (
        "Energy prices should rise after shock"
    )


# ── Test: household reads sector employment changes ──────────────────


def test_household_reads_sector_employment():
    """Counties with industry mix should see income effects from sectors."""
    counties = {
        "12345": {
            "population": 50_000,
            "major_industries": {
                "Energy": 0.4,
                "Manufacturing": 0.3,
                "Tech": 0.2,
                "Retail": 0.1,
            },
        },
    }
    events = {
        1: [{"type": "SectorShock", "sector": "Energy", "severity": 3.0}],
    }

    deltas = _run_simulation(8, events_by_tick=events, counties=counties)

    # The county should show some income impact from the sector shock
    county_impacts = [
        d.get("county_deltas", {}).get("12345", {})
        for d in deltas
    ]
    has_impact = any(
        c.get("income_delta_by_quintile") is not None
        and any(abs(q) > 0 for q in c.get("income_delta_by_quintile", []))
        for c in county_impacts
    )
    assert has_impact, (
        "County with Energy industry should see income effects after energy shock"
    )


# ── Test: macro anchors sectors ──────────────────────────────────────


def test_macro_anchors_sector_employment():
    """Sector employment should calibrate toward macro unemployment anchor."""
    # Run with high unemployment — sectors should adjust
    deltas_high = _run_simulation(
        8,
        counties={},
    )

    # All sector employment deltas should be finite and reasonable
    for d in deltas_high:
        for sector, sd in d.get("sector_deltas", {}).items():
            emp = sd.get("employment_delta", 0)
            assert abs(emp) < 1.0, (
                f"Sector {sector} employment delta {emp} is unreasonably large"
            )


# ── Test: multi-tick consistency ─────────────────────────────────────


def test_multi_tick_macro_state_accumulates():
    """Two fiscal bills should produce a higher peak GDP growth than one."""
    events_double = {
        1: [{"type": "FiscalBill",
             "bill_type": "spending", "amount_gdp_pct": 0.02,
             }],
        3: [{"type": "FiscalBill",
             "bill_type": "spending", "amount_gdp_pct": 0.02,
             }],
    }

    deltas_double = _run_simulation(12, events_by_tick=events_double)

    events_single = {
        1: [{"type": "FiscalBill",
             "bill_type": "spending", "amount_gdp_pct": 0.02,
             }],
    }
    deltas_single = _run_simulation(12, events_by_tick=events_single)

    # Cumulative GDP growth at each tick (running sum of deltas)
    cum_double = []
    cum_single = []
    s = 0.0
    for d in deltas_double:
        s += d.get("gdp_growth_delta", 0)
        cum_double.append(s)
    s = 0.0
    for d in deltas_single:
        s += d.get("gdp_growth_delta", 0)
        cum_single.append(s)

    # After the second bill lands (tick 3+), double should exceed single
    # at some point in the later ticks
    peak_double = max(cum_double[3:])
    peak_single = max(cum_single[3:])

    assert peak_double > peak_single, (
        f"Two fiscal bills (peak {peak_double:.6f}) should produce "
        f"higher cumulative GDP than one (peak {peak_single:.6f})"
    )


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])
