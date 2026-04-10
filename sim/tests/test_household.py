"""Tests for the household microsimulation layer."""

from sim.layers.household import HouseholdLayer


def _world_state(counties=None, macro=None):
    return {
        "week": 1,
        "counties": counties or {},
        "macro": {
            "gdp": 27_000_000_000_000,
            "gdp_growth": 0.02,
            "inflation": 0.03,
            "unemployment": 0.04,
            "fed_funds_rate": 0.05,
            "consumer_confidence": 100.0,
            "debt_to_gdp": 1.2,
            **(macro or {}),
        },
    }


def _default_delta():
    return {
        "county_deltas": {},
        "narrative_seeds": [],
    }


def _fiscal_event(bill_type="tax_cut", amount_gdp_pct=0.01,
                   affected_counties=None, sector=None):
    event = {
        "type": "FiscalBill",
        "bill_type": bill_type,
        "amount_gdp_pct": amount_gdp_pct,
    }
    if affected_counties is not None:
        event["affected_counties"] = affected_counties
    if sector is not None:
        event["sector"] = sector
    return event


class TestBaselineNoEvents:
    def test_no_events_returns_zero_deltas(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        result = layer.step(_world_state(), [], delta)
        assert result["county_deltas"] == {}
        assert result["narrative_seeds"] == []

    def test_non_fiscal_events_ignored(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        events = [{"type": "SectorShock", "sector": "Energy", "severity": 0.5}]
        result = layer.step(_world_state(), events, delta)
        assert result["county_deltas"] == {}


class TestTaxCut:
    def test_tax_cut_increases_income(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        events = [_fiscal_event("tax_cut", 0.01, ["OH-001"])]
        ws = _world_state(counties={"OH-001": {"population": 100_000}})
        result = layer.step(ws, events, delta)

        cd = result["county_deltas"]["OH-001"]
        quintiles = cd["income_delta_by_quintile"]
        # All quintiles should get positive income
        assert all(q > 0 for q in quintiles)

    def test_tax_cut_top_quintile_benefits_more(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        events = [_fiscal_event("tax_cut", 0.01, ["OH-001"])]
        ws = _world_state(counties={"OH-001": {"population": 100_000}})
        result = layer.step(ws, events, delta)

        quintiles = result["county_deltas"]["OH-001"]["income_delta_by_quintile"]
        # Top quintile (index 4) should get more than bottom (index 0)
        assert quintiles[4] > quintiles[0]
        # Ratio should be 0.45/0.05 = 9x
        assert abs(quintiles[4] / quintiles[0] - 9.0) < 0.01


class TestSpending:
    def test_spending_benefits_lower_quintiles_more(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        events = [_fiscal_event("spending", 0.01, ["OH-001"])]
        ws = _world_state(counties={"OH-001": {"population": 100_000}})
        result = layer.step(ws, events, delta)

        quintiles = result["county_deltas"]["OH-001"]["income_delta_by_quintile"]
        # Bottom quintile should get more than top
        assert quintiles[0] > quintiles[4]

    def test_spending_generates_narrative(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        events = [_fiscal_event("spending", 0.01, ["OH-001"])]
        ws = _world_state(counties={"OH-001": {"population": 100_000}})
        result = layer.step(ws, events, delta)

        assert len(result["narrative_seeds"]) > 0
        assert "bottom quintile" in result["narrative_seeds"][0].lower()


class TestCaching:
    def test_results_cached_between_unchanged_ticks(self):
        layer = HouseholdLayer()
        ws = _world_state(counties={"OH-001": {"population": 100_000}})
        events = [_fiscal_event("tax_cut", 0.01, ["OH-001"])]

        delta1 = _default_delta()
        result1 = layer.step(ws, events, delta1)

        delta2 = _default_delta()
        result2 = layer.step(ws, events, delta2)

        # Should produce identical results from cache
        q1 = result1["county_deltas"]["OH-001"]["income_delta_by_quintile"]
        q2 = result2["county_deltas"]["OH-001"]["income_delta_by_quintile"]
        assert q1 == q2

    def test_cache_invalidated_on_new_events(self):
        layer = HouseholdLayer()
        ws = _world_state(counties={"OH-001": {"population": 100_000}})

        delta1 = _default_delta()
        events1 = [_fiscal_event("tax_cut", 0.01, ["OH-001"])]
        layer.step(ws, events1, delta1)

        delta2 = _default_delta()
        events2 = [_fiscal_event("spending", 0.02, ["OH-001"])]
        result2 = layer.step(ws, events2, delta2)

        # Spending: bottom > top
        q = result2["county_deltas"]["OH-001"]["income_delta_by_quintile"]
        assert q[0] > q[4]

    def test_no_events_after_events_clears_results(self):
        layer = HouseholdLayer()
        ws = _world_state(counties={"OH-001": {"population": 100_000}})

        delta1 = _default_delta()
        events1 = [_fiscal_event("tax_cut", 0.01, ["OH-001"])]
        layer.step(ws, events1, delta1)

        delta2 = _default_delta()
        result2 = layer.step(ws, [], delta2)
        assert result2["county_deltas"] == {}


class TestBenefitEligibility:
    def test_high_unemployment_increases_snap_eligibility(self):
        layer = HouseholdLayer()
        delta = _default_delta()
        events = [_fiscal_event("spending", 0.001, ["OH-001"])]
        ws = _world_state(
            counties={"OH-001": {"population": 100_000}},
            macro={"unemployment": 0.10},
        )
        result = layer.step(ws, events, delta)

        cd = result["county_deltas"]["OH-001"]
        assert cd["snap_eligible_change"] > 0
        assert cd["medicaid_eligible_change"] > 0
