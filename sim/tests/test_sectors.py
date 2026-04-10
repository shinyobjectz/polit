"""Tests for the SectorLayer and SectorAgent."""

from __future__ import annotations

import pytest

from sim.agents.sector_agent import SectorAgent
from sim.layers.sectors import SectorLayer, SectorModel, SECTOR_NAMES


# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------

def _empty_delta() -> dict:
    return {"sector_deltas": {}, "narrative_seeds": []}


# ------------------------------------------------------------------
# Baseline initialisation
# ------------------------------------------------------------------

class TestBaseline:
    def test_nine_sectors_created(self):
        model = SectorModel()
        assert len(model.agents) == 9

    def test_sector_names(self):
        model = SectorModel()
        names = sorted(a.sector_name for a in model.agents)
        assert names == sorted(SECTOR_NAMES)

    def test_baseline_values(self):
        """All sectors start at 1.0 for output, employment, and price."""
        model = SectorModel()
        for agent in model.agents:
            assert agent.output == pytest.approx(1.0)
            assert agent.employment == pytest.approx(1.0)
            assert agent.price_level == pytest.approx(1.0)


# ------------------------------------------------------------------
# GDP growth effects
# ------------------------------------------------------------------

class TestGDPGrowth:
    def test_positive_gdp_increases_demand_and_output(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.10, "fed_funds_rate": 0.02}}
        delta = _empty_delta()

        delta = layer.step(world, [], delta)

        for name, vals in delta["sector_deltas"].items():
            assert vals["output_delta"] > 0, f"{name} output should rise"

    def test_high_interest_rate_dampens_demand(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.02, "fed_funds_rate": 0.20}}
        delta = _empty_delta()

        delta = layer.step(world, [], delta)

        for name, vals in delta["sector_deltas"].items():
            assert vals["output_delta"] < 0, f"{name} output should fall"


# ------------------------------------------------------------------
# Sector shock
# ------------------------------------------------------------------

class TestSectorShock:
    def test_energy_shock_reduces_output(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.02, "fed_funds_rate": 0.05}}
        shock = [{"type": "SectorShock", "sector": "Energy", "severity": 0.5}]

        # Apply shock for several ticks so the supply drop feeds through
        delta = _empty_delta()
        for _ in range(5):
            delta = _empty_delta()
            delta = layer.step(world, shock, delta)

        energy = delta["sector_deltas"]["Energy"]
        # The sustained supply hit should push prices up relative to
        # non-shocked sectors (supply drops while demand stays the same).
        assert energy["price_delta"] > 0, "Energy prices should rise after shock"

    def test_energy_shock_does_not_affect_tech(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.02, "fed_funds_rate": 0.05}}
        shock = [{"type": "SectorShock", "sector": "Energy", "severity": 0.5}]

        delta = _empty_delta()
        for _ in range(5):
            delta = _empty_delta()
            delta = layer.step(world, shock, delta)

        tech = delta["sector_deltas"]["Tech"]
        energy = delta["sector_deltas"]["Energy"]
        # Energy should have a much larger price increase than Tech
        assert energy["price_delta"] > tech["price_delta"]


# ------------------------------------------------------------------
# Tariff effects
# ------------------------------------------------------------------

class TestTariff:
    def test_tariff_raises_prices_in_affected_sectors(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.02, "fed_funds_rate": 0.05}}
        events = [{"type": "Tariff", "rate": 0.20}]

        delta = _empty_delta()
        delta = layer.step(world, events, delta)

        for name in ("Manufacturing", "Tech", "Agriculture"):
            assert delta["sector_deltas"][name]["price_delta"] > 0, (
                f"{name} prices should rise from tariff"
            )

    def test_tariff_does_not_affect_unrelated_sectors(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.02, "fed_funds_rate": 0.05}}
        events = [{"type": "Tariff", "rate": 0.20}]

        delta = _empty_delta()
        delta = layer.step(world, events, delta)

        # Healthcare should not see a tariff price bump
        # With neutral macro, price_delta should be near zero
        healthcare_price = delta["sector_deltas"]["Healthcare"]["price_delta"]
        mfg_price = delta["sector_deltas"]["Manufacturing"]["price_delta"]
        assert mfg_price > healthcare_price


# ------------------------------------------------------------------
# Convergence over multiple ticks
# ------------------------------------------------------------------

class TestConvergence:
    def test_output_converges_to_demand(self):
        """After many ticks with stable macro, output should approach demand."""
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.05, "fed_funds_rate": 0.03}}

        delta = _empty_delta()
        for _ in range(50):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        for agent in layer._model.agents:
            assert agent.output == pytest.approx(agent.demand, abs=0.01)

    def test_employment_tracks_output(self):
        """Employment converges toward output over many ticks."""
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.05, "fed_funds_rate": 0.03}}

        for _ in range(100):
            delta = _empty_delta()
            layer.step(world, [], delta)

        for agent in layer._model.agents:
            assert agent.employment == pytest.approx(agent.output, abs=0.01)

    def test_supply_converges_to_output(self):
        """Supply converges toward output over many ticks."""
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.05, "fed_funds_rate": 0.03}}

        for _ in range(200):
            delta = _empty_delta()
            layer.step(world, [], delta)

        for agent in layer._model.agents:
            assert agent.supply == pytest.approx(agent.output, abs=0.01)


# ------------------------------------------------------------------
# Narrative seeds
# ------------------------------------------------------------------

class TestNarrativeSeeds:
    def test_significant_change_generates_narrative(self):
        layer = SectorLayer()
        world = {"macro": {"gdp_growth": 0.20, "fed_funds_rate": 0.01}}

        # Run enough ticks to push output far from 1.0
        delta = _empty_delta()
        for _ in range(10):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        assert len(delta["narrative_seeds"]) > 0
        assert any("expanding" in s for s in delta["narrative_seeds"])
