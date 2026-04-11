"""Tests for the CorporateLayer and CorporateAgent."""

from __future__ import annotations

import pytest

from sim.agents.corporate_agent import CorporateAgent
from sim.layers.corporate import (
    CorporateLayer,
    CorporateModel,
    SECTOR_PROFILES,
)


# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------

def _empty_delta() -> dict:
    return {"corporate_actions": [], "narrative_seeds": []}


def _bill_event(bill_type: str, sector_target: str = "") -> dict:
    return {"type": "FiscalBill", "bill_type": bill_type, "sector_target": sector_target}


# ------------------------------------------------------------------
# Baseline initialisation
# ------------------------------------------------------------------

class TestBaseline:
    def test_nine_sectors_created(self):
        model = CorporateModel()
        assert len(model.agents) == 9

    def test_all_sectors_initialized_with_correct_interests(self):
        """Every sector profile from the GDD table is present with matching attributes."""
        model = CorporateModel()
        agents_by_sector = {a.sector: a for a in model.agents}

        for profile in SECTOR_PROFILES:
            agent = agents_by_sector[profile["sector"]]
            assert agent.wants == profile["wants"]
            assert agent.opposes == profile["opposes"]
            assert agent.lobby_intensity == profile["lobby_intensity"]
            assert agent.donation_pattern == profile["donation_pattern"]
            assert agent.leverage == 1.0


# ------------------------------------------------------------------
# Policy impact scoring
# ------------------------------------------------------------------

class TestPolicyImpact:
    def test_wanted_policy_positive(self):
        """A bill matching 'wants' yields positive impact."""
        model = CorporateModel()
        energy = next(a for a in model.agents if a.sector == "Energy")
        assert energy.compute_policy_impact("deregulation", "") == pytest.approx(0.5)

    def test_opposed_policy_negative(self):
        """A bill matching 'opposes' yields negative impact."""
        model = CorporateModel()
        energy = next(a for a in model.agents if a.sector == "Energy")
        assert energy.compute_policy_impact("carbon_tax", "") == pytest.approx(-0.5)

    def test_sector_targeted_doubles_impact(self):
        """A bill targeting the agent's own sector doubles the effect."""
        model = CorporateModel()
        energy = next(a for a in model.agents if a.sector == "Energy")
        assert energy.compute_policy_impact("deregulation", "Energy") == pytest.approx(1.0)

    def test_sector_targeted_negative_clamped(self):
        """Negative impact doubled is clamped to -1.0."""
        model = CorporateModel()
        energy = next(a for a in model.agents if a.sector == "Energy")
        assert energy.compute_policy_impact("carbon_tax", "Energy") == pytest.approx(-1.0)

    def test_irrelevant_policy_zero(self):
        """A bill not in wants or opposes yields zero."""
        model = CorporateModel()
        energy = next(a for a in model.agents if a.sector == "Energy")
        assert energy.compute_policy_impact("some_random_policy", "") == pytest.approx(0.0)


# ------------------------------------------------------------------
# Reaction matrix
# ------------------------------------------------------------------

class TestReactionMatrix:
    def test_tax_cut_energy_positive_reaction(self):
        """Tax cut (wanted by Energy) → positive reaction → donation."""
        layer = CorporateLayer()
        delta = _empty_delta()
        events = [_bill_event("tax_breaks")]

        delta = layer.step({}, events, delta)

        energy_actions = [a for a in delta["corporate_actions"] if a["sector"] == "Energy"]
        assert len(energy_actions) > 0
        action_types = {a["type"] for a in energy_actions}
        # impact=0.5, lobby=0.8, leverage=1.0 → intensity=0.4 → major_donation
        assert "major_donation" in action_types

    def test_carbon_tax_energy_negative_reaction(self):
        """Carbon tax (opposed by Energy) → negative reaction → lobby against."""
        layer = CorporateLayer()
        delta = _empty_delta()
        events = [_bill_event("carbon_tax")]

        delta = layer.step({}, events, delta)

        energy_actions = [a for a in delta["corporate_actions"] if a["sector"] == "Energy"]
        assert len(energy_actions) > 0
        action_types = {a["type"] for a in energy_actions}
        # impact=-0.5, lobby=0.8, leverage=1.0 → intensity=-0.4 → attack_ads + fund_opposition
        assert "attack_ads" in action_types or "lobby_against" in action_types

    def test_tariff_manufacturing_positive(self):
        """Tariff (wanted by Manufacturing) → positive reaction."""
        layer = CorporateLayer()
        delta = _empty_delta()
        events = [_bill_event("tariffs")]

        delta = layer.step({}, events, delta)

        mfg_actions = [a for a in delta["corporate_actions"] if a["sector"] == "Manufacturing"]
        assert len(mfg_actions) > 0
        # All manufacturing actions should have positive intensity
        assert all(a["intensity"] > 0 for a in mfg_actions)

    def test_large_negative_triggers_nuclear_response(self):
        """Impact <= -0.5 triggers plant closure / legal challenge."""
        layer = CorporateLayer()
        delta = _empty_delta()
        # Target Energy specifically with an opposed policy → impact = -1.0
        events = [_bill_event("carbon_tax", "Energy")]

        delta = layer.step({}, events, delta)

        energy_actions = [a for a in delta["corporate_actions"] if a["sector"] == "Energy"]
        action_types = {a["type"] for a in energy_actions}
        assert "threaten_plant_closure" in action_types
        assert "legal_challenge" in action_types

    def test_corporate_actions_include_target_and_intensity(self):
        """Every corporate action has sector and intensity fields."""
        layer = CorporateLayer()
        delta = _empty_delta()
        events = [_bill_event("deregulation")]

        delta = layer.step({}, events, delta)

        assert len(delta["corporate_actions"]) > 0
        for action in delta["corporate_actions"]:
            assert "sector" in action
            assert "intensity" in action
            assert "type" in action
            assert isinstance(action["intensity"], float)


# ------------------------------------------------------------------
# Leverage updates
# ------------------------------------------------------------------

class TestLeverage:
    def test_leverage_increases_with_sector_output(self):
        """When sector output delta is positive, leverage rises."""
        layer = CorporateLayer()
        delta = _empty_delta()
        delta["sector_deltas"] = {"Energy": {"output_delta": 0.3}}

        layer.step({}, [], delta)

        agent = layer._agent_by_sector("Energy")
        assert agent is not None
        assert agent.leverage == pytest.approx(1.3)

    def test_leverage_clamped_to_range(self):
        """Leverage is clamped to [0.5, 2.0]."""
        layer = CorporateLayer()
        delta = _empty_delta()
        delta["sector_deltas"] = {"Energy": {"output_delta": 5.0}}

        layer.step({}, [], delta)

        agent = layer._agent_by_sector("Energy")
        assert agent is not None
        assert agent.leverage == pytest.approx(2.0)


# ------------------------------------------------------------------
# Narrative seeds
# ------------------------------------------------------------------

class TestNarrativeSeeds:
    def test_nuclear_action_generates_narrative(self):
        """Plant closure / legal challenge actions produce narrative seeds."""
        layer = CorporateLayer()
        delta = _empty_delta()
        events = [_bill_event("carbon_tax", "Energy")]

        delta = layer.step({}, events, delta)

        assert any("plant closure" in s for s in delta["narrative_seeds"])

    def test_no_events_no_narratives(self):
        """No FiscalBill events → no corporate narrative seeds."""
        layer = CorporateLayer()
        delta = _empty_delta()

        delta = layer.step({}, [], delta)

        # Only corporate-originated seeds; list should be empty
        corp_seeds = [s for s in delta["narrative_seeds"] if "sector" in s.lower()]
        assert len(corp_seeds) == 0
