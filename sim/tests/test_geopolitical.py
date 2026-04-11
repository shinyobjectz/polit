"""Tests for the GeopoliticalLayer and CountryAgent."""

from __future__ import annotations

import pytest

from sim.agents.country_agent import CountryAgent, compute_migration_pressure
from sim.layers.geopolitical import (
    GeopoliticalLayer,
    GeopoliticalModel,
    TIER_1_COUNTRIES,
    TIER_2_COUNTRIES,
    ALL_DEFAULT_COUNTRIES,
)


# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------

def _empty_delta() -> dict:
    return {"narrative_seeds": []}


def _layer() -> GeopoliticalLayer:
    return GeopoliticalLayer()


# ------------------------------------------------------------------
# Default countries & tier assignments
# ------------------------------------------------------------------

class TestDefaults:
    def test_total_country_count(self):
        model = GeopoliticalModel()
        assert len(model.agents) == 12  # 6 tier-1 + 6 tier-2

    def test_tier_1_countries(self):
        model = GeopoliticalModel()
        tier1 = [a for a in model.agents if a.tier == 1]
        names = sorted(a.name for a in tier1)
        assert names == sorted(["China", "Russia", "UK", "EU", "Israel", "Iran"])

    def test_tier_2_countries(self):
        model = GeopoliticalModel()
        tier2 = [a for a in model.agents if a.tier == 2]
        names = sorted(a.name for a in tier2)
        expected = sorted([
            "Japan", "India", "Saudi Arabia", "Mexico", "Brazil", "South Korea",
        ])
        assert names == expected

    def test_tier_assignments_correct(self):
        for spec in ALL_DEFAULT_COUNTRIES:
            if spec["name"] in {"China", "Russia", "UK", "EU", "Israel", "Iran"}:
                assert spec["tier"] == 1, f"{spec['name']} should be tier 1"
            else:
                assert spec["tier"] == 2, f"{spec['name']} should be tier 2"


# ------------------------------------------------------------------
# Tariff on China reduces trade, shifts alignment negative
# ------------------------------------------------------------------

class TestTariff:
    def test_tariff_reduces_trade(self):
        layer = _layer()
        model = layer._model
        china = model.get_country("China")
        original_imports = china.trade_with_us["imports"]
        original_exports = china.trade_with_us["exports"]

        events = [{"type": "Tariff", "target": "China", "rate": 0.25}]
        layer.step({}, events, _empty_delta())

        assert china.trade_with_us["imports"] < original_imports
        assert china.trade_with_us["exports"] < original_exports

    def test_tariff_shifts_alignment_negative(self):
        layer = _layer()
        china = layer._model.get_country("China")
        original_alignment = china.alignment

        events = [{"type": "Tariff", "target": "China", "rate": 0.25}]
        layer.step({}, events, _empty_delta())

        assert china.alignment < original_alignment


# ------------------------------------------------------------------
# Sanction on Russia affects trade volumes
# ------------------------------------------------------------------

class TestSanction:
    def test_sanction_reduces_trade(self):
        layer = _layer()
        russia = layer._model.get_country("Russia")
        original_imports = russia.trade_with_us["imports"]
        original_exports = russia.trade_with_us["exports"]

        events = [{"type": "Sanction", "target": "Russia", "severity": 0.8}]
        layer.step({}, events, _empty_delta())

        assert russia.trade_with_us["imports"] < original_imports
        assert russia.trade_with_us["exports"] < original_exports

    def test_sanction_shifts_alignment_negative(self):
        layer = _layer()
        russia = layer._model.get_country("Russia")
        original_alignment = russia.alignment

        events = [{"type": "Sanction", "target": "Russia", "severity": 0.8}]
        layer.step({}, events, _empty_delta())

        assert russia.alignment < original_alignment


# ------------------------------------------------------------------
# Alliance strengthening with UK increases alignment
# ------------------------------------------------------------------

class TestAllianceShift:
    def test_alliance_shift_increases_alignment(self):
        layer = _layer()
        uk = layer._model.get_country("UK")
        original_alignment = uk.alignment

        events = [{"type": "AllianceShift", "country": "UK", "shift": 0.1}]
        layer.step({}, events, _empty_delta())

        # Account for the 0.999 drift in step()
        assert uk.alignment > original_alignment

    def test_alliance_clamped_at_one(self):
        layer = _layer()
        events = [{"type": "AllianceShift", "country": "UK", "shift": 5.0}]
        layer.step({}, events, _empty_delta())

        uk = layer._model.get_country("UK")
        assert uk.alignment <= 1.0


# ------------------------------------------------------------------
# Migration pressure increases when stability drops
# ------------------------------------------------------------------

class TestMigrationPressure:
    def test_low_stability_increases_pressure(self):
        layer = _layer()
        iran = layer._model.get_country("Iran")
        uk = layer._model.get_country("UK")
        # Iran (stability 45) should have more pressure than UK (stability 85)
        pressure_iran = compute_migration_pressure(iran)
        pressure_uk = compute_migration_pressure(uk)
        assert pressure_iran > pressure_uk

    def test_zero_pressure_when_stable(self):
        import mesa
        model = mesa.Model()
        country = CountryAgent(
            model,
            name="Stable",
            tier=3,
            alignment=0.5,
            military=10,
            economic=50,
            diplomatic=50,
            nuclear=0,
            stability=80,
            trade_with_us={"imports": 10, "exports": 10},
        )
        assert compute_migration_pressure(country) == 0.0

    def test_high_pressure_when_unstable(self):
        import mesa
        model = mesa.Model()
        country = CountryAgent(
            model,
            name="Unstable",
            tier=3,
            alignment=-0.5,
            military=10,
            economic=20,
            diplomatic=10,
            nuclear=0,
            stability=10,
            trade_with_us={"imports": 0, "exports": 0},
        )
        pressure = compute_migration_pressure(country)
        assert pressure > 1.0

    def test_migration_in_delta(self):
        layer = _layer()
        delta = _empty_delta()
        layer.step({}, [], delta)
        assert "migration_pressure" in delta
        assert "Iran" in delta["migration_pressure"]
        assert delta["migration_pressure"]["Iran"] > 0


# ------------------------------------------------------------------
# Conflict reduces stability of involved parties
# ------------------------------------------------------------------

class TestConflict:
    def test_conflict_reduces_stability(self):
        layer = _layer()
        russia = layer._model.get_country("Russia")
        original_stability = russia.stability

        events = [{"type": "Conflict", "countries": ["Russia"], "severity": 0.5}]
        layer.step({}, events, _empty_delta())

        assert russia.stability < original_stability

    def test_conflict_shifts_alignment_negative(self):
        layer = _layer()
        russia = layer._model.get_country("Russia")
        original_alignment = russia.alignment

        events = [{"type": "Conflict", "countries": ["Russia"], "severity": 0.5}]
        layer.step({}, events, _empty_delta())

        assert russia.alignment < original_alignment

    def test_conflict_multi_country(self):
        layer = _layer()
        events = [{
            "type": "Conflict",
            "countries": ["Russia", "Iran"],
            "severity": 0.8,
        }]
        delta = _empty_delta()
        layer.step({}, events, delta)

        russia = layer._model.get_country("Russia")
        iran = layer._model.get_country("Iran")
        # Both should have reduced stability
        assert russia.stability < 55  # original was 55
        assert iran.stability < 45  # original was 45


# ------------------------------------------------------------------
# Agent slow drift between events
# ------------------------------------------------------------------

class TestAgentDrift:
    def test_alignment_drifts_toward_zero(self):
        import mesa
        model = mesa.Model()
        agent = CountryAgent(
            model,
            name="Test",
            tier=1,
            alignment=0.5,
            military=50,
            economic=50,
            diplomatic=50,
            nuclear=0,
            stability=50,
            trade_with_us={"imports": 0, "exports": 0},
        )
        agent.step()
        assert abs(agent.alignment) < 0.5

    def test_stability_mean_reverts(self):
        import mesa
        model = mesa.Model()
        agent = CountryAgent(
            model,
            name="Test",
            tier=1,
            alignment=0.0,
            military=50,
            economic=50,
            diplomatic=50,
            nuclear=0,
            stability=80,
            trade_with_us={"imports": 0, "exports": 0},
        )
        agent.step()
        assert agent.stability < 80  # reverts toward 50


# ------------------------------------------------------------------
# Trade balance delta
# ------------------------------------------------------------------

class TestTradeBalance:
    def test_trade_balance_in_delta(self):
        layer = _layer()
        delta = _empty_delta()
        layer.step({}, [], delta)
        assert "trade_balance_delta" in delta
        # US imports more than it exports in defaults -> deficit
        assert delta["trade_balance_delta"] < 0

    def test_narrative_seeds_on_events(self):
        layer = _layer()
        delta = _empty_delta()
        events = [{"type": "Tariff", "target": "China", "rate": 0.25}]
        layer.step({}, events, delta)
        assert len(delta["narrative_seeds"]) > 0
