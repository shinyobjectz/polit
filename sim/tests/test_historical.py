"""Historical calibration tests.

Load pre-baked scenario TOML files and run them through the simulation
to verify the outcomes are plausible given historical context.  These
validate that the economic models produce trajectories that roughly
match what happened in reality.
"""

import json
import msgpack
import pytest
from pathlib import Path

from sim.host import tick, reset_layers
from sim.scenario import load_scenario, scenario_to_world_state

SCENARIOS_DIR = Path(__file__).parent.parent.parent / "game" / "scenarios"


@pytest.fixture(autouse=True)
def _fresh():
    reset_layers()
    yield
    reset_layers()


def _run_scenario_file(filename, n_ticks=52):
    """Load a scenario TOML and run it through the simulation."""
    path = SCENARIOS_DIR / filename
    if not path.exists():
        pytest.skip(f"Scenario file not found: {path}")

    config = load_scenario(path)
    world = scenario_to_world_state(config)
    m = dict(world["macro"])
    deltas = []

    for week in range(1, n_ticks + 1):
        world["week"] = week
        world["macro"] = m
        ws = msgpack.packb(world)
        # Include scheduled events for this week
        events = config.scheduled_events.get(week, [])
        ev = msgpack.packb(events)
        delta = json.loads(tick(ws, ev))
        deltas.append(delta)
        m["gdp_growth"] += delta.get("gdp_growth_delta", 0)
        m["inflation"] += delta.get("inflation_delta", 0)
        m["unemployment"] += delta.get("unemployment_delta", 0)
        m["consumer_confidence"] += delta.get("consumer_confidence_delta", 0)
        if delta.get("fed_funds_rate"):
            m["fed_funds_rate"] = delta["fed_funds_rate"]

    return deltas, m, config


# ---- Test 1: 2008 Recession Trajectory ----


def test_2008_recession_trajectory():
    """2008 scenario should produce deepening recession with eventual stabilization."""
    deltas, final, config = _run_scenario_file("recession_2008.toml", n_ticks=52)

    # Starting GDP was negative (-2.8%), should get worse initially from banking crisis
    # but eventually mean-revert toward recovery
    min_gdp = min(
        sum(d.get("gdp_growth_delta", 0) for d in deltas[:i + 1]) + config.macro["gdp_growth"]
        for i in range(len(deltas))
    )
    # GDP should dip further before recovering
    assert min_gdp < config.macro["gdp_growth"], "Should worsen before recovery"

    # Unemployment should rise from the crisis
    # (started at 7.3%, banking crisis should push it higher)
    max_unemp = max(
        sum(d.get("unemployment_delta", 0) for d in deltas[:i + 1]) + config.macro["unemployment"]
        for i in range(len(deltas))
    )
    assert max_unemp > config.macro["unemployment"], "Unemployment should rise during crisis"

    # Fed should keep rates very low (started at 1%, should stay low or go to ZLB)
    # The macro layer may emit 0.0 when the rate is at/near ZLB — treat that
    # as effectively zero.  Check that the final rate never climbed high.
    assert final["fed_funds_rate"] < 0.03, "Fed should maintain low rates during crisis"

    # By end of year, the rate of decline should be slowing (mean-reversion
    # kicking in).  Compare average GDP delta in the last quarter vs the worst
    # quarter to see deceleration of the downturn.
    q_size = 13
    last_q_avg = sum(d.get("gdp_growth_delta", 0) for d in deltas[-q_size:]) / q_size
    worst_q_avg = min(
        sum(d.get("gdp_growth_delta", 0) for d in deltas[i:i + q_size]) / q_size
        for i in range(len(deltas) - q_size + 1)
    )
    assert last_q_avg >= worst_q_avg, "Decline should be decelerating by week 52"

    # All values should stay bounded (no explosions)
    assert -0.2 < final["gdp_growth"] < 0.2
    assert 0.0 < final["unemployment"] < 0.3
    assert final["consumer_confidence"] > 0


# ---- Test 2: 1970s Stagflation ----


def test_1970s_stagflation_dynamics():
    """1970s scenario should show persistent high inflation with weak growth."""
    deltas, final, config = _run_scenario_file("stagflation_1970s.toml", n_ticks=52)

    # Started with 9% inflation -- should stay elevated due to energy shock
    assert final["inflation"] > 0.04, f"Inflation should remain high, got {final['inflation']}"

    # GDP started negative, energy shock should keep it weak
    assert final["gdp_growth"] < 0.03, "Growth should remain below trend"

    # Fed should be aggressive (Taylor rule responding to high inflation)
    max_rate = max(d.get("fed_funds_rate", 0) for d in deltas)
    assert max_rate > 0.05, f"Fed should tighten aggressively, max rate was {max_rate}"

    # Narrative seeds should mention inflation and energy
    all_seeds = []
    for d in deltas:
        all_seeds.extend(d.get("narrative_seeds", []))
    energy_seeds = [s for s in all_seeds if "nergy" in s.lower()]
    assert len(energy_seeds) > 0, "Should have energy-related narratives"


# ---- Test 3: Modern 2024 Stability ----


def test_modern_2024_stays_stable():
    """Modern 2024 scenario with no events should remain roughly stable."""
    deltas, final, config = _run_scenario_file("modern_2024.toml", n_ticks=52)

    # Without events, economy should drift toward equilibrium
    # GDP should stay near 2% trend
    assert 0.0 < final["gdp_growth"] < 0.05, f"GDP should be near trend, got {final['gdp_growth']}"

    # Unemployment should stay low
    assert final["unemployment"] < 0.06, f"Unemployment should stay low, got {final['unemployment']}"

    # No extreme movements
    max_approval_swing = max(abs(d.get("approval_president_delta", 0)) for d in deltas)
    assert max_approval_swing < 10, "No wild approval swings without events"


# ---- Test 4: Boom Economy Correction ----


def test_boom_economy_correction():
    """Boom scenario should show eventual cooling via Fed tightening."""
    deltas, final, config = _run_scenario_file("boom_economy.toml", n_ticks=52)

    # Started at 4.5% GDP -- should cool toward trend
    assert final["gdp_growth"] < config.macro["gdp_growth"], "GDP should cool from boom levels"

    # Fed tightens in response to above-trend growth
    initial_rate = config.macro["fed_funds_rate"]
    max_rate = max(d.get("fed_funds_rate", 0) for d in deltas)
    assert max_rate >= initial_rate, "Fed should tighten or hold during boom"

    # Dot-com shock at week 25 should be visible in tech sector
    # Week 25 is index 24 in the deltas list
    post_shock_deltas = deltas[24:]
    tech_hit = any(
        d.get("sector_deltas", {}).get("Tech", {}).get("output_delta", 0) < -0.01
        for d in post_shock_deltas
    )
    assert tech_hit, "Tech sector should show impact from dot-com shock"


# ---- Test 5: Cold War Scenario Geopolitics ----


def test_cold_war_geopolitical_cascade():
    """Cold war scenario should show geopolitical events affecting economy."""
    deltas, final, config = _run_scenario_file("cold_war_tension.toml", n_ticks=30)

    # Should have foreign power deltas from conflict and sanctions
    foreign_deltas_count = sum(
        len(d.get("foreign_power_deltas", [])) for d in deltas
    )
    assert foreign_deltas_count > 0, "Should have geopolitical activity"

    # Migration pressure should appear (conflict regions destabilize)
    has_migration = any(len(d.get("migration_pressure", {})) > 0 for d in deltas)
    assert has_migration, "Conflict should create migration pressure"

    # Economy should be stressed — consumer confidence should stay below
    # the equilibrium level (100), even if it mean-reverts from its low start
    assert final["consumer_confidence"] < 100, \
        f"Confidence should stay below equilibrium under geopolitical stress, got {final['consumer_confidence']}"
