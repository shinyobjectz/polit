"""Tests for the MarketLayer, MarketModel, and market agents."""

from __future__ import annotations

import pytest

from sim.agents.market_agent import BondAgent, CommodityAgent, SectorIndexAgent
from sim.layers.markets import MarketLayer, MarketModel, SECTOR_NAMES


# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------

def _empty_delta() -> dict:
    return {"narrative_seeds": []}


def _default_world() -> dict:
    return {
        "macro": {
            "gdp_growth": 0.02,
            "fed_funds_rate": 0.025,
            "consumer_confidence": 100.0,
            "debt_to_gdp": 1.0,
        }
    }


# ------------------------------------------------------------------
# Baseline initialisation
# ------------------------------------------------------------------

class TestBaseline:
    def test_thirteen_agents_created(self):
        model = MarketModel()
        assert len(model.agents) == 13  # 9 sectors + 3 commodities + 1 bond

    def test_sector_index_names(self):
        model = MarketModel()
        names = sorted(
            a.sector_name for a in model.agents if isinstance(a, SectorIndexAgent)
        )
        assert names == sorted(SECTOR_NAMES)

    def test_initial_sector_prices(self):
        model = MarketModel()
        for agent in model.agents:
            if isinstance(agent, SectorIndexAgent):
                assert agent.price == pytest.approx(100.0)

    def test_initial_oil_price(self):
        model = MarketModel()
        for agent in model.agents:
            if isinstance(agent, CommodityAgent) and agent.commodity_name == "oil":
                assert agent.price == pytest.approx(80.0)

    def test_initial_bond_yield(self):
        model = MarketModel()
        for agent in model.agents:
            if isinstance(agent, BondAgent):
                assert agent.yield_rate == pytest.approx(0.04)


# ------------------------------------------------------------------
# Sector shock produces immediate price drop
# ------------------------------------------------------------------

class TestSectorShock:
    def test_sector_shock_drops_price(self):
        layer = MarketLayer()
        world = _default_world()
        events = [{"type": "SectorShock", "sector": "Tech", "severity": 0.5}]

        delta = _empty_delta()
        delta = layer.step(world, events, delta)

        tech_price = delta["sector_indices"]["Tech"]
        assert tech_price < 100.0, "Tech index should drop after sector shock"

    def test_sector_shock_does_not_affect_other_sectors(self):
        layer = MarketLayer()
        world = _default_world()
        events = [{"type": "SectorShock", "sector": "Tech", "severity": 0.5}]

        delta = _empty_delta()
        delta = layer.step(world, events, delta)

        tech_price = delta["sector_indices"]["Tech"]
        energy_price = delta["sector_indices"]["Energy"]
        # Energy shouldn't be hit; Tech should be lower
        assert tech_price < energy_price


# ------------------------------------------------------------------
# Fed rate hike increases bond yield
# ------------------------------------------------------------------

class TestBondYield:
    def test_fed_rate_hike_increases_yield(self):
        layer = MarketLayer()

        # Step with low rate first to establish baseline
        world_low = _default_world()
        world_low["macro"]["fed_funds_rate"] = 0.01
        delta = _empty_delta()
        for _ in range(20):
            delta = _empty_delta()
            delta = layer.step(world_low, [], delta)
        low_yield = delta["bond_yield_10yr"]

        # Now hike the rate
        world_high = _default_world()
        world_high["macro"]["fed_funds_rate"] = 0.06
        for _ in range(20):
            delta = _empty_delta()
            delta = layer.step(world_high, [], delta)
        high_yield = delta["bond_yield_10yr"]

        assert high_yield > low_yield, "Bond yield should rise with fed rate"

    def test_yield_never_negative(self):
        layer = MarketLayer()
        world = _default_world()
        world["macro"]["fed_funds_rate"] = 0.0
        world["macro"]["debt_to_gdp"] = 0.5

        delta = _empty_delta()
        for _ in range(100):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        assert delta["bond_yield_10yr"] >= 0.0


# ------------------------------------------------------------------
# Oil reacts to GDP growth
# ------------------------------------------------------------------

class TestOilPrice:
    def test_high_gdp_raises_oil(self):
        layer = MarketLayer()
        world = _default_world()
        world["macro"]["gdp_growth"] = 0.06  # well above baseline 0.02

        delta = _empty_delta()
        for _ in range(10):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        assert delta["oil_price"] > 80.0, "Oil should rise with high GDP growth"

    def test_low_gdp_lowers_oil(self):
        layer = MarketLayer()
        world = _default_world()
        world["macro"]["gdp_growth"] = -0.02  # recession

        delta = _empty_delta()
        for _ in range(10):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        assert delta["oil_price"] < 80.0, "Oil should fall with negative GDP"


# ------------------------------------------------------------------
# Mean reversion toward fundamentals
# ------------------------------------------------------------------

class TestMeanReversion:
    def test_sector_index_reverts_after_shock(self):
        layer = MarketLayer()
        world = _default_world()

        # Apply a shock
        events = [{"type": "SectorShock", "sector": "Finance", "severity": 0.8}]
        delta = _empty_delta()
        delta = layer.step(world, events, delta)
        shocked_price = delta["sector_indices"]["Finance"]

        # Let it recover with no further shocks
        for _ in range(50):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)
        recovered_price = delta["sector_indices"]["Finance"]

        assert recovered_price > shocked_price, "Price should recover toward fundamental"

    def test_commodity_reverts_to_base(self):
        layer = MarketLayer()
        world = _default_world()

        # Shock oil via sanction
        events = [{"type": "Sanction", "severity": 1.0}]
        delta = _empty_delta()
        delta = layer.step(world, events, delta)
        spiked = delta["oil_price"]

        # Let it revert
        for _ in range(50):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)
        reverted = delta["oil_price"]

        assert abs(reverted - 80.0) < abs(spiked - 80.0), (
            "Oil should revert toward base price"
        )


# ------------------------------------------------------------------
# Market sentiment correlates with consumer confidence
# ------------------------------------------------------------------

class TestSentiment:
    def test_high_confidence_boosts_prices(self):
        layer_high = MarketLayer()
        world_high = _default_world()
        world_high["macro"]["consumer_confidence"] = 120.0

        layer_low = MarketLayer()
        world_low = _default_world()
        world_low["macro"]["consumer_confidence"] = 80.0

        delta_high = _empty_delta()
        delta_low = _empty_delta()
        for _ in range(10):
            delta_high = _empty_delta()
            delta_low = _empty_delta()
            delta_high = layer_high.step(world_high, [], delta_high)
            delta_low = layer_low.step(world_low, [], delta_low)

        # Average sector index should be higher with high confidence
        avg_high = sum(delta_high["sector_indices"].values()) / 9
        avg_low = sum(delta_low["sector_indices"].values()) / 9
        assert avg_high > avg_low, "Higher confidence should boost index prices"


# ------------------------------------------------------------------
# Sector indices are always > 0
# ------------------------------------------------------------------

class TestPriceFloor:
    def test_sector_indices_always_positive(self):
        layer = MarketLayer()
        world = _default_world()
        world["macro"]["consumer_confidence"] = 20.0  # very low
        world["macro"]["gdp_growth"] = -0.10  # deep recession

        for _ in range(100):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        for name, price in delta["sector_indices"].items():
            assert price > 0, f"{name} index must be positive, got {price}"

    def test_oil_always_positive(self):
        layer = MarketLayer()
        world = _default_world()
        world["macro"]["gdp_growth"] = -0.10

        for _ in range(100):
            delta = _empty_delta()
            delta = layer.step(world, [], delta)

        assert delta["oil_price"] > 0, "Oil price must be positive"


# ------------------------------------------------------------------
# Narrative seeds for significant moves
# ------------------------------------------------------------------

class TestNarrativeSeeds:
    def test_large_shock_generates_narrative(self):
        layer = MarketLayer()
        world = _default_world()
        events = [{"type": "SectorShock", "sector": "Energy", "severity": 1.0}]

        delta = _empty_delta()
        delta = layer.step(world, events, delta)

        assert any(
            "Energy" in s and "dropped" in s for s in delta["narrative_seeds"]
        ), "Large shock should generate narrative seed"
