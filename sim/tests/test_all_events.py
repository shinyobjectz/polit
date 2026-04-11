"""Round-trip tests for all 12 SimEvent variants through the full simulation pipeline.

Sends every SimEvent variant through tick() and verifies a valid delta comes back,
proving Rust→Python event serialization works for all event types.
"""

import json
import math

import msgpack
import pytest

from sim.host import tick, reset_layers


@pytest.fixture(autouse=True)
def _fresh():
    reset_layers()
    yield
    reset_layers()


WORLD_STATE = msgpack.packb({
    "week": 1,
    "macro": {
        "gdp_growth": 0.02,
        "inflation": 0.02,
        "unemployment": 0.045,
        "fed_funds_rate": 0.025,
        "consumer_confidence": 100.0,
        "debt_to_gdp": 1.0,
    },
    "counties": {},
})

REQUIRED_KEYS = {
    "gdp_growth_delta",
    "inflation_delta",
    "unemployment_delta",
    "fed_funds_rate",
    "consumer_confidence_delta",
    "debt_to_gdp_delta",
    "sector_deltas",
    "county_deltas",
    "approval_president_delta",
    "approval_congress_delta",
    "protest_risk_by_region",
    "voter_ideology_shifts",
    "sector_indices",
    "oil_price",
    "bond_yield_10yr",
    "foreign_power_deltas",
    "trade_balance_delta",
    "migration_pressure",
    "corporate_actions",
    "narrative_seeds",
}

FLOAT_KEYS = [
    "gdp_growth_delta",
    "inflation_delta",
    "unemployment_delta",
    "fed_funds_rate",
    "consumer_confidence_delta",
    "approval_president_delta",
    "oil_price",
    "bond_yield_10yr",
    "trade_balance_delta",
]


def _send_event(event):
    """Send a single event through the pipeline and return the delta dict."""
    ev = msgpack.packb([event])
    result = tick(WORLD_STATE, ev)
    delta = json.loads(result)
    return delta


def _assert_valid_delta(delta):
    """Check delta has all required keys and no NaN/Inf."""
    for key in REQUIRED_KEYS:
        assert key in delta, f"Missing key: {key}"
    for key in FLOAT_KEYS:
        val = delta.get(key, 0)
        if isinstance(val, float):
            assert not math.isnan(val), f"NaN in {key}"
            assert not math.isinf(val), f"Inf in {key}"


# ── One test per SimEvent variant (flat "type" format) ───────────────────


def test_fiscal_bill():
    delta = _send_event({
        "type": "FiscalBill",
        "bill_type": "spending",
        "amount_gdp_pct": 0.02,
        "sector": None,
        "distributional_target": None,
    })
    _assert_valid_delta(delta)


def test_monetary_policy():
    delta = _send_event({
        "type": "MonetaryPolicy",
        "fed_funds_delta": 0.25,
    })
    _assert_valid_delta(delta)


def test_economy_shock():
    delta = _send_event({
        "type": "EconomyShock",
        "shock_type": "demand_collapse",
        "magnitude": 2.0,
        "duration_weeks": 10,
    })
    _assert_valid_delta(delta)


def test_sector_shock():
    delta = _send_event({
        "type": "SectorShock",
        "sector": "Energy",
        "region": "Gulf",
        "severity": 1.5,
    })
    _assert_valid_delta(delta)
    assert len(delta["sector_deltas"]) > 0, "SectorShock should produce sector deltas"


def test_tariff():
    delta = _send_event({
        "type": "Tariff",
        "partner": "China",
        "product": "electronics",
        "rate": 0.25,
    })
    _assert_valid_delta(delta)


def test_scandal():
    delta = _send_event({
        "type": "Scandal",
        "actor": "Senator",
        "severity": 3.0,
        "scandal_type": "financial",
        "media_amplification": 1.5,
    })
    _assert_valid_delta(delta)
    assert delta["approval_president_delta"] < 0, "Scandal should reduce approval"


def test_protest():
    delta = _send_event({
        "type": "Protest",
        "protest_type": "economic",
        "scale": 2.0,
        "region": "national",
        "police_response": "restrained",
    })
    _assert_valid_delta(delta)


def test_media_campaign():
    delta = _send_event({
        "type": "MediaCampaign",
        "campaign_type": "advertising",
        "target_group": "all",
        "intensity": 0.8,
        "source": "domestic",
    })
    _assert_valid_delta(delta)


def test_natural_disaster():
    delta = _send_event({
        "type": "NaturalDisaster",
        "disaster_type": "hurricane",
        "region": "Southeast",
        "severity": 3.0,
    })
    _assert_valid_delta(delta)


def test_conflict():
    delta = _send_event({
        "type": "Conflict",
        "parties": ["Iran"],
        "conflict_type": "military",
        "escalation_level": 0.5,
    })
    _assert_valid_delta(delta)
    assert len(delta["foreign_power_deltas"]) > 0, \
        "Conflict should produce foreign power deltas"


def test_sanction():
    delta = _send_event({
        "type": "Sanction",
        "target_country": "Russia",
        "sector": None,
        "intensity": 0.7,
    })
    _assert_valid_delta(delta)


def test_alliance_shift():
    delta = _send_event({
        "type": "AllianceShift",
        "country": "UK",
        "direction": "closer",
        "domain": "military",
    })
    _assert_valid_delta(delta)


# ── Serde-tagged format (what Rust actually sends) ───────────────────────


def test_serde_tagged_fiscal_bill():
    """Rust sends externally-tagged enums: {"FiscalBill": {fields...}}"""
    delta = _send_event({
        "FiscalBill": {
            "bill_type": "tax_cut",
            "amount_gdp_pct": 0.01,
            "sector": None,
            "distributional_target": None,
        },
    })
    _assert_valid_delta(delta)


def test_serde_tagged_sector_shock():
    delta = _send_event({
        "SectorShock": {
            "sector": "Tech",
            "region": None,
            "severity": 1.0,
        },
    })
    _assert_valid_delta(delta)
    assert len(delta["sector_deltas"]) > 0


def test_serde_tagged_conflict():
    delta = _send_event({
        "Conflict": {
            "parties": ["China"],
            "conflict_type": "economic",
            "escalation_level": 0.3,
        },
    })
    _assert_valid_delta(delta)
