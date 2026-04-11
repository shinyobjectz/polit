"""Tests for the PoliticalLayer and VoterAgent opinion dynamics."""

from __future__ import annotations

import pytest

from sim.agents.voter_agent import VoterAgent
from sim.layers.political import PoliticalLayer, PoliticalModel


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_layer() -> PoliticalLayer:
    return PoliticalLayer()


def _default_delta() -> dict:
    return {
        "gdp_growth_delta": 0.0,
        "inflation_delta": 0.0,
        "unemployment_delta": 0.0,
        "consumer_confidence_delta": 0.0,
        "approval_president_delta": 0.0,
        "approval_congress_delta": 0.0,
        "protest_risk_by_region": {},
        "voter_ideology_shifts": [],
        "narrative_seeds": [],
        "sector_deltas": {},
        "county_deltas": {},
    }


# ---------------------------------------------------------------------------
# Economic recession shifts approval negative
# ---------------------------------------------------------------------------

class TestRecessionApproval:
    def test_high_unemployment_hurts_approval(self):
        layer = _make_layer()
        world = {
            "macro": {
                "gdp_growth": -0.01,  # contraction
                "unemployment": 0.10,  # 10%
                "consumer_confidence": 60,
            }
        }
        delta = _default_delta()
        delta = layer.step(world, [], delta)

        # Approval should be pushed negative
        assert delta["approval_president_delta"] < 0
        assert delta["approval_congress_delta"] < 0

    def test_normal_economy_minimal_approval_change(self):
        layer = _make_layer()
        world = {
            "macro": {
                "gdp_growth": 0.02,
                "unemployment": 0.04,
                "consumer_confidence": 100,
            }
        }
        delta = _default_delta()
        delta = layer.step(world, [], delta)

        # Near-zero anxiety -> near-zero approval impact
        assert abs(delta["approval_president_delta"]) < 0.5


# ---------------------------------------------------------------------------
# Scandal event reduces trust_institutions across all groups
# ---------------------------------------------------------------------------

class TestScandalTrust:
    def test_scandal_reduces_trust(self):
        layer = _make_layer()
        world = {"macro": {}}
        events = [{"type": "Scandal", "severity": 5}]
        delta = _default_delta()

        # Capture trust before
        trust_before = [
            a.trust_institutions for a in layer._model.agents
        ]

        layer.step(world, events, delta)

        for agent, tb in zip(layer._model.agents, trust_before):
            assert agent.trust_institutions < tb

    def test_scandal_trust_floors_at_zero(self):
        layer = _make_layer()
        # Set trust very low
        for agent in layer._model.agents:
            agent.trust_institutions = 0.01

        events = [{"type": "Scandal", "severity": 10}]
        delta = _default_delta()
        layer.step({"macro": {}}, events, delta)

        for agent in layer._model.agents:
            assert agent.trust_institutions >= 0.0

    def test_serde_tagged_scandal(self):
        """Scandals arriving in serde-tagged format are handled."""
        layer = _make_layer()
        events = [{"Scandal": {"severity": 3}}]
        trust_before = [a.trust_institutions for a in layer._model.agents]
        delta = _default_delta()
        layer.step({"macro": {}}, events, delta)

        for agent, tb in zip(layer._model.agents, trust_before):
            assert agent.trust_institutions < tb


# ---------------------------------------------------------------------------
# Ideology drift is bounded [-1, 1] and small per tick
# ---------------------------------------------------------------------------

class TestIdeologyBounds:
    def test_ideology_bounded_after_many_ticks(self):
        layer = _make_layer()
        world = {
            "macro": {
                "gdp_growth": -0.05,
                "unemployment": 0.15,
            }
        }
        delta = _default_delta()

        for _ in range(500):
            delta = _default_delta()
            layer.step(world, [], delta)

        for agent in layer._model.agents:
            assert -1.0 <= agent.ideology_score <= 1.0

    def test_ideology_drift_small_per_tick(self):
        layer = _make_layer()
        scores_before = [a.ideology_score for a in layer._model.agents]

        world = {
            "macro": {"gdp_growth": -0.01, "unemployment": 0.08}
        }
        delta = _default_delta()
        layer.step(world, [], delta)

        for agent, before in zip(layer._model.agents, scores_before):
            drift = abs(agent.ideology_score - before)
            assert drift < 0.05  # small per tick


# ---------------------------------------------------------------------------
# Protest risk increases with high unemployment + low trust
# ---------------------------------------------------------------------------

class TestProtestRisk:
    def test_protest_risk_elevated_in_crisis(self):
        layer = _make_layer()
        # Tank trust
        for agent in layer._model.agents:
            agent.trust_institutions = 0.1

        world = {
            "macro": {
                "gdp_growth": -0.03,
                "unemployment": 0.12,
            }
        }
        delta = _default_delta()
        delta = layer.step(world, [], delta)

        assert "national" in delta["protest_risk_by_region"]
        assert delta["protest_risk_by_region"]["national"] > 0.1

    def test_no_protest_risk_in_good_times(self):
        layer = _make_layer()
        world = {
            "macro": {
                "gdp_growth": 0.03,
                "unemployment": 0.035,
            }
        }
        delta = _default_delta()
        delta = layer.step(world, [], delta)

        assert delta["protest_risk_by_region"].get("national", 0) <= 0.1


# ---------------------------------------------------------------------------
# Turnout propensity stays within [0.2, 0.95]
# ---------------------------------------------------------------------------

class TestTurnoutBounds:
    def test_turnout_clamped_high(self):
        layer = _make_layer()
        # Max out trust so anxiety * (trust - 0.5) is large
        for agent in layer._model.agents:
            agent.trust_institutions = 1.0

        world = {
            "macro": {
                "gdp_growth": -0.05,
                "unemployment": 0.20,
            }
        }
        delta = _default_delta()

        for _ in range(100):
            delta = _default_delta()
            layer.step(world, [], delta)

        for agent in layer._model.agents:
            assert agent.turnout_propensity <= 0.95

    def test_turnout_clamped_low(self):
        layer = _make_layer()
        # Zero trust -> anxiety pushes turnout down
        for agent in layer._model.agents:
            agent.trust_institutions = 0.0

        world = {
            "macro": {
                "gdp_growth": -0.05,
                "unemployment": 0.20,
            }
        }
        delta = _default_delta()

        for _ in range(100):
            delta = _default_delta()
            layer.step(world, [], delta)

        for agent in layer._model.agents:
            assert agent.turnout_propensity >= 0.2


# ---------------------------------------------------------------------------
# Protest event increases turnout
# ---------------------------------------------------------------------------

class TestProtestEvent:
    def test_protest_boosts_turnout(self):
        layer = _make_layer()

        # First step with no events to establish baseline turnout
        delta = _default_delta()
        layer.step({"macro": {}}, [], delta)
        turnout_baseline = [a.turnout_propensity for a in layer._model.agents]

        # Now step with a protest event (same macro conditions)
        events = [{"type": "Protest", "scale": 5, "region": "midwest"}]
        delta = _default_delta()
        layer.step({"macro": {}}, events, delta)

        for agent, tb in zip(layer._model.agents, turnout_baseline):
            assert agent.turnout_propensity >= tb


# ---------------------------------------------------------------------------
# VoterAgent unit tests
# ---------------------------------------------------------------------------

class TestVoterAgentUnit:
    def test_default_construction(self):
        model = PoliticalModel()
        agent = VoterAgent(
            model,
            county_fips="12345",
            demographic="test_group",
            population_share=0.25,
        )
        assert agent.county_fips == "12345"
        assert agent.demographic == "test_group"
        assert agent.population_share == 0.25
        assert agent.ideology_score == 0.0
        assert agent.economic_anxiety == 0.0

    def test_custom_ideology(self):
        model = PoliticalModel()
        agent = VoterAgent(
            model,
            county_fips="00000",
            demographic="left_leaning",
            population_share=0.10,
            ideology_score=-0.5,
        )
        assert agent.ideology_score == -0.5


# ---------------------------------------------------------------------------
# PoliticalModel initialises with 5 default demographic groups
# ---------------------------------------------------------------------------

class TestPoliticalModel:
    def test_default_agents_created(self):
        model = PoliticalModel()
        agents = list(model.agents)
        assert len(agents) == 5

        demographics = {a.demographic for a in agents}
        expected = {
            "young_college",
            "young_nocollege",
            "middle_age",
            "older_working",
            "retired",
        }
        assert demographics == expected

    def test_population_shares_sum_to_one(self):
        model = PoliticalModel()
        total = sum(a.population_share for a in model.agents)
        assert abs(total - 1.0) < 1e-9
