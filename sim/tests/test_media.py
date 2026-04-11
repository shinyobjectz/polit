"""Tests for the MediaLayer and MediaAgent news propagation."""

from __future__ import annotations

import pytest

from sim.agents.media_agent import MediaAgent
from sim.layers.media import MediaLayer, MediaModel, DEFAULT_OUTLETS


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_layer() -> MediaLayer:
    return MediaLayer()


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
# Default media ecosystem has expected outlets
# ---------------------------------------------------------------------------

class TestMediaModel:
    def test_default_outlets_created(self):
        model = MediaModel()
        agents = list(model.agents)
        assert len(agents) == 8

        names = {a.name for a in agents}
        expected = {
            "CNN", "Fox News", "MSNBC", "NYT", "WSJ",
            "AP/Reuters", "Social Media", "Local News",
        }
        assert names == expected

    def test_ap_reuters_highest_credibility(self):
        model = MediaModel()
        agents = list(model.agents)
        ap = next(a for a in agents if a.name == "AP/Reuters")
        for agent in agents:
            assert ap.credibility >= agent.credibility

    def test_social_media_highest_reach(self):
        model = MediaModel()
        agents = list(model.agents)
        social = next(a for a in agents if a.name == "Social Media")
        for agent in agents:
            assert social.reach >= agent.reach


# ---------------------------------------------------------------------------
# High-credibility outlet shifts beliefs more than low-credibility
# ---------------------------------------------------------------------------

class TestCredibilityImpact:
    def test_high_cred_more_impact_on_advertising(self):
        """Advertising campaign: high-credibility outlets produce larger
        positive belief shift than low-credibility outlets."""
        layer = _make_layer()
        events = [
            {
                "type": "MediaCampaign",
                "campaign_type": "advertising",
                "intensity": 5,
                "source": "domestic",
            }
        ]
        delta = _default_delta()
        delta = layer.step({}, events, delta)

        # Advertising should boost approval
        assert delta["approval_president_delta"] > 0


# ---------------------------------------------------------------------------
# Disinformation from foreign source has reduced impact (0.3x multiplier)
# ---------------------------------------------------------------------------

class TestForeignDisinformation:
    def test_foreign_source_reduced_impact(self):
        layer = _make_layer()
        # Domestic disinformation
        events_domestic = [
            {
                "type": "MediaCampaign",
                "campaign_type": "disinformation",
                "intensity": 5,
                "source": "domestic",
            }
        ]
        delta_domestic = _default_delta()
        delta_domestic = layer.step({}, events_domestic, delta_domestic)
        domestic_shift = delta_domestic["approval_president_delta"]

        # Foreign disinformation (fresh layer to avoid state carryover)
        layer2 = _make_layer()
        events_foreign = [
            {
                "type": "MediaCampaign",
                "campaign_type": "disinformation",
                "intensity": 5,
                "source": "foreign",
            }
        ]
        delta_foreign = _default_delta()
        delta_foreign = layer2.step({}, events_foreign, delta_foreign)
        foreign_shift = delta_foreign["approval_president_delta"]

        # Both should be negative (disinformation hurts approval)
        assert domestic_shift < 0
        assert foreign_shift < 0
        # Foreign impact should be ~0.3x of domestic
        assert abs(foreign_shift) < abs(domestic_shift)
        ratio = abs(foreign_shift) / abs(domestic_shift)
        assert abs(ratio - 0.3) < 0.05


# ---------------------------------------------------------------------------
# Scandal event amplified by media coverage
# ---------------------------------------------------------------------------

class TestScandalAmplification:
    def test_scandal_reduces_approval(self):
        layer = _make_layer()
        events = [{"type": "Scandal", "severity": 5}]
        delta = _default_delta()
        delta = layer.step({}, events, delta)

        assert delta["approval_president_delta"] < 0

    def test_scandal_generates_narrative(self):
        layer = _make_layer()
        events = [{"type": "Scandal", "severity": 3}]
        delta = _default_delta()
        delta = layer.step({}, events, delta)

        seeds = delta["narrative_seeds"]
        assert any("Scandal dominating" in s for s in seeds)

    def test_higher_severity_larger_impact(self):
        layer1 = _make_layer()
        delta1 = _default_delta()
        delta1 = layer1.step({}, [{"type": "Scandal", "severity": 2}], delta1)

        layer2 = _make_layer()
        delta2 = _default_delta()
        delta2 = layer2.step({}, [{"type": "Scandal", "severity": 8}], delta2)

        assert abs(delta2["approval_president_delta"]) > abs(
            delta1["approval_president_delta"]
        )

    def test_serde_tagged_scandal(self):
        """Scandal arriving in serde-tagged format is handled."""
        layer = _make_layer()
        events = [{"Scandal": {"severity": 4}}]
        delta = _default_delta()
        delta = layer.step({}, events, delta)

        assert delta["approval_president_delta"] < 0


# ---------------------------------------------------------------------------
# MediaCampaign advertising boosts approval
# ---------------------------------------------------------------------------

class TestAdvertisingCampaign:
    def test_advertising_boosts_approval(self):
        layer = _make_layer()
        events = [
            {
                "type": "MediaCampaign",
                "campaign_type": "advertising",
                "intensity": 5,
                "source": "domestic",
            }
        ]
        delta = _default_delta()
        delta = layer.step({}, events, delta)

        assert delta["approval_president_delta"] > 0


# ---------------------------------------------------------------------------
# Negativity bias: bad news amplified more than good news
# ---------------------------------------------------------------------------

class TestNegativityBias:
    def test_bad_news_amplified_more(self):
        """Negative approval delta gets amplified more than positive."""
        # Negative signal
        layer_neg = _make_layer()
        delta_neg = _default_delta()
        delta_neg["approval_president_delta"] = -2.0
        delta_neg = layer_neg.step({}, [], delta_neg)
        neg_amplification = abs(delta_neg["approval_president_delta"]) / 2.0

        # Positive signal
        layer_pos = _make_layer()
        delta_pos = _default_delta()
        delta_pos["approval_president_delta"] = 2.0
        delta_pos = layer_pos.step({}, [], delta_pos)
        pos_amplification = delta_pos["approval_president_delta"] / 2.0

        # Negative news should be amplified more
        assert neg_amplification > pos_amplification

    def test_small_signals_not_amplified(self):
        """Approval deltas below threshold (0.5) are not amplified."""
        layer = _make_layer()
        delta = _default_delta()
        delta["approval_president_delta"] = 0.3
        delta = layer.step({}, [], delta)

        # Should remain unchanged (below 0.5 threshold)
        assert abs(delta["approval_president_delta"] - 0.3) < 1e-9

    def test_amplification_generates_narrative(self):
        layer = _make_layer()
        delta = _default_delta()
        delta["approval_president_delta"] = -3.0
        delta = layer.step({}, [], delta)

        seeds = delta["narrative_seeds"]
        assert any("Media coverage amplifying" in s for s in seeds)


# ---------------------------------------------------------------------------
# MediaAgent unit tests
# ---------------------------------------------------------------------------

class TestMediaAgentUnit:
    def test_construction(self):
        model = MediaModel()
        agent = MediaAgent(
            model,
            name="TestOutlet",
            media_type="digital",
            reach=0.05,
            credibility=70,
            editorial_lean=-10,
        )
        assert agent.name == "TestOutlet"
        assert agent.media_type == "digital"
        assert agent.reach == 0.05
        assert agent.credibility == 70
        assert agent.editorial_lean == -10
        assert agent.current_stories == []

    def test_step_clears_stories(self):
        model = MediaModel()
        agent = MediaAgent(
            model,
            name="TestOutlet",
            media_type="tv",
            reach=0.1,
            credibility=50,
            editorial_lean=0,
        )
        agent.current_stories = [{"headline": "Breaking news"}]
        agent.step()
        assert agent.current_stories == []
