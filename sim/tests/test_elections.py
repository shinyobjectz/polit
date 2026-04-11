"""Tests for the election input computation module."""

from __future__ import annotations

import types

import pytest

from sim.layers.elections import (
    ElectionInputs,
    compute_election_inputs,
    election_inputs_to_dict,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_agent(
    county_fips: str = "12345",
    population_share: float = 0.5,
    ideology_score: float = 0.0,
    turnout_propensity: float = 0.60,
) -> types.SimpleNamespace:
    """Create a lightweight stand-in for a VoterAgent."""
    return types.SimpleNamespace(
        county_fips=county_fips,
        population_share=population_share,
        ideology_score=ideology_score,
        turnout_propensity=turnout_propensity,
    )


def _base_world(
    gdp_growth: float = 0.02,
    unemployment: float = 0.045,
    inflation: float = 0.02,
    counties: dict | None = None,
) -> dict:
    if counties is None:
        counties = {
            "12345": {"unemployment_rate": unemployment},
        }
    return {
        "macro": {
            "gdp_growth": gdp_growth,
            "unemployment": unemployment,
            "inflation": inflation,
        },
        "counties": counties,
    }


# ---------------------------------------------------------------------------
# Economic recession increases incumbent vulnerability (lower approval)
# ---------------------------------------------------------------------------


class TestApprovalDuringRecession:
    def test_recession_lowers_approval(self) -> None:
        normal = compute_election_inputs(_base_world(), {})
        recession = compute_election_inputs(
            _base_world(gdp_growth=-0.01), {"gdp_growth_delta": 0}
        )
        assert recession.approval_rating["12345"] < normal.approval_rating["12345"]

    def test_deep_recession_more_severe(self) -> None:
        mild = compute_election_inputs(
            _base_world(gdp_growth=0.01), {}
        )
        deep = compute_election_inputs(
            _base_world(gdp_growth=-0.03), {}
        )
        assert deep.approval_rating["12345"] < mild.approval_rating["12345"]


# ---------------------------------------------------------------------------
# High unemployment increases economy/jobs issue salience
# ---------------------------------------------------------------------------


class TestIssueSalience:
    def test_high_unemployment_raises_economy_salience(self) -> None:
        low = compute_election_inputs(
            _base_world(unemployment=0.04), {}
        )
        high = compute_election_inputs(
            _base_world(unemployment=0.10), {}
        )
        assert high.issue_salience["economy"] > low.issue_salience["economy"]

    def test_high_unemployment_raises_jobs_salience(self) -> None:
        low = compute_election_inputs(
            _base_world(unemployment=0.04), {}
        )
        high = compute_election_inputs(
            _base_world(unemployment=0.10), {}
        )
        assert high.issue_salience["jobs"] > low.issue_salience["jobs"]

    def test_salience_values_in_range(self) -> None:
        """All issue salience values must be in [0, 1]."""
        for unemp in (0.02, 0.045, 0.10, 0.20):
            for infl in (0.01, 0.05, 0.10):
                inputs = compute_election_inputs(
                    _base_world(unemployment=unemp, inflation=infl), {}
                )
                for topic, val in inputs.issue_salience.items():
                    assert 0.0 <= val <= 1.0, (
                        f"{topic} salience {val} out of range "
                        f"(unemp={unemp}, infl={infl})"
                    )


# ---------------------------------------------------------------------------
# Swing county identification
# ---------------------------------------------------------------------------


class TestSwingCounties:
    def test_near_zero_ideology_is_swing(self) -> None:
        agents = [
            _make_agent("A", 0.5, ideology_score=-0.05),
            _make_agent("A", 0.5, ideology_score=0.05),
        ]
        world = _base_world(counties={"A": {}})
        inputs = compute_election_inputs(world, {}, voter_agents=agents)
        assert "A" in inputs.swing_counties

    def test_strong_leaning_not_swing(self) -> None:
        agents = [
            _make_agent("B", 0.5, ideology_score=0.5),
            _make_agent("B", 0.5, ideology_score=0.6),
        ]
        world = _base_world(counties={"B": {}})
        inputs = compute_election_inputs(world, {}, voter_agents=agents)
        assert "B" not in inputs.swing_counties


# ---------------------------------------------------------------------------
# Turnout propensity in valid range [0, 1]
# ---------------------------------------------------------------------------


class TestTurnoutRange:
    def test_with_agents(self) -> None:
        agents = [
            _make_agent("C", 0.5, turnout_propensity=0.3),
            _make_agent("C", 0.5, turnout_propensity=0.9),
        ]
        world = _base_world(counties={"C": {}})
        inputs = compute_election_inputs(world, {}, voter_agents=agents)
        for fips, tp in inputs.turnout_propensity.items():
            assert 0.0 <= tp <= 1.0, f"Turnout {tp} out of range for {fips}"

    def test_fallback_turnout(self) -> None:
        inputs = compute_election_inputs(_base_world(), {})
        for fips, tp in inputs.turnout_propensity.items():
            assert 0.0 <= tp <= 1.0


# ---------------------------------------------------------------------------
# With voter agents: ideology distribution sums to ~1.0 per county
# ---------------------------------------------------------------------------


class TestIdeologyDistribution:
    def test_sums_to_one(self) -> None:
        agents = [
            _make_agent("D", 0.3, ideology_score=-0.5),
            _make_agent("D", 0.3, ideology_score=0.0),
            _make_agent("D", 0.4, ideology_score=0.5),
        ]
        world = _base_world(counties={"D": {}})
        inputs = compute_election_inputs(world, {}, voter_agents=agents)
        dist = inputs.ideology_distribution["D"]
        total = dist["left"] + dist["center"] + dist["right"]
        assert abs(total - 1.0) < 1e-9, f"Distribution sums to {total}"


# ---------------------------------------------------------------------------
# Without voter agents: fallback produces valid generic estimates
# ---------------------------------------------------------------------------


class TestFallback:
    def test_fallback_ideology(self) -> None:
        inputs = compute_election_inputs(_base_world(), {})
        dist = inputs.ideology_distribution["12345"]
        assert dist["left"] == pytest.approx(0.35)
        assert dist["center"] == pytest.approx(0.30)
        assert dist["right"] == pytest.approx(0.35)

    def test_fallback_turnout_value(self) -> None:
        inputs = compute_election_inputs(_base_world(), {})
        assert inputs.turnout_propensity["12345"] == pytest.approx(0.60)


# ---------------------------------------------------------------------------
# Enthusiasm gap is negative during recession (opposition advantage)
# ---------------------------------------------------------------------------


class TestEnthusiasmGap:
    def test_negative_during_recession(self) -> None:
        """High unemployment -> high anxiety -> negative enthusiasm gap."""
        inputs = compute_election_inputs(
            _base_world(unemployment=0.10), {}
        )
        assert inputs.enthusiasm_gap < 0.0

    def test_near_zero_in_good_times(self) -> None:
        """Low unemployment -> low anxiety -> near-zero gap."""
        inputs = compute_election_inputs(
            _base_world(unemployment=0.04), {}
        )
        assert abs(inputs.enthusiasm_gap) < 0.05


# ---------------------------------------------------------------------------
# Serialization
# ---------------------------------------------------------------------------


class TestSerialization:
    def test_round_trip_keys(self) -> None:
        inputs = compute_election_inputs(_base_world(), {})
        d = election_inputs_to_dict(inputs)
        expected_keys = {
            "ideology_distribution",
            "turnout_propensity",
            "economic_conditions",
            "approval_rating",
            "issue_salience",
            "swing_counties",
            "enthusiasm_gap",
        }
        assert set(d.keys()) == expected_keys

    def test_serializable_types(self) -> None:
        """All values should be JSON/msgpack-friendly primitives."""
        inputs = compute_election_inputs(_base_world(), {})
        d = election_inputs_to_dict(inputs)
        assert isinstance(d["enthusiasm_gap"], float)
        assert isinstance(d["swing_counties"], list)
        assert isinstance(d["issue_salience"], dict)
