"""Simplified household microsimulation layer.

Translates fiscal policy events (tax cuts, spending bills, mandates,
regulations) into household-level income effects by income quintile and
county, with simplified benefit-eligibility tracking (SNAP, Medicaid).
"""

from __future__ import annotations

from sim.layers.base import SimulationLayer

# Income quintile distribution weights ---------------------------------
# How a $1 of policy impact is distributed across quintiles (Q1=lowest).
TAX_CUT_WEIGHTS = [0.05, 0.10, 0.15, 0.25, 0.45]
SPENDING_WEIGHTS = [0.35, 0.25, 0.20, 0.12, 0.08]
MANDATE_WEIGHTS = [0.20, 0.20, 0.20, 0.20, 0.20]  # uniform
REGULATION_WEIGHTS = [0.10, 0.15, 0.20, 0.25, 0.30]

BILL_TYPE_WEIGHTS: dict[str, list[float]] = {
    "tax_cut": TAX_CUT_WEIGHTS,
    "spending": SPENDING_WEIGHTS,
    "mandate": MANDATE_WEIGHTS,
    "regulation": REGULATION_WEIGHTS,
}

# Benefit thresholds (annual household income, family of 4, 2024) ------
SNAP_THRESHOLD = 36_000  # ~130 % FPL
MEDICAID_THRESHOLD = 38_300  # ~138 % FPL

# Baseline US GDP used when world_state doesn't supply one.
_DEFAULT_GDP = 27_0000_0000_0000  # ~$27 T


class HouseholdLayer(SimulationLayer):
    """Simplified household microsimulation.

    Computes disposable-income effects from policy changes using
    simplified tax/benefit rules inspired by PolicyEngine's approach.

    Results are cached and only recalculated when new ``FiscalBill``
    events arrive or macro conditions shift significantly.
    """

    def __init__(self) -> None:
        self._cache_key: str | None = None
        self._cached_results: dict | None = None

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        fiscal_events = [
            e for e in events
            if _event_type(e) == "FiscalBill"
        ]

        macro = world_state.get("macro", {})
        # Effective macro values incorporating current-tick macro layer deltas
        unemployment = (
            macro.get("unemployment", 0.04)
            + delta.get("unemployment_delta", 0.0)
        )
        gdp_growth = (
            macro.get("gdp_growth", 0.02)
            + delta.get("gdp_growth_delta", 0.0)
        )
        gdp = macro.get("gdp", _DEFAULT_GDP)
        counties = world_state.get("counties", {})

        # Read sector deltas produced by the sector layer this tick
        sector_deltas = delta.get("sector_deltas", {})

        # Invalidate cache when macro conditions shift or sectors move
        has_sector_movement = any(
            abs(sd.get("employment_delta", 0)) > 0.001
            for sd in sector_deltas.values()
        )
        cache_key = _build_cache_key(fiscal_events, macro)

        if (
            cache_key == self._cache_key
            and self._cached_results is not None
            and not has_sector_movement
        ):
            _merge_cached(delta, self._cached_results)
            return delta

        county_deltas: dict[str, dict] = {}
        narrative_seeds: list[str] = []

        # ── 1. Apply sector employment changes to county incomes ─────
        for county_id, county_data in counties.items():
            industries = county_data.get("major_industries", {})
            if not industries and not sector_deltas:
                continue

            # Weight sector employment changes by county industry mix
            county_employment_impact = sum(
                sector_deltas.get(sector, {}).get("employment_delta", 0.0)
                * share
                for sector, share in industries.items()
            )

            if abs(county_employment_impact) > 1e-6:
                existing = county_deltas.get(county_id, {
                    "income_delta_by_quintile": [0.0] * 5,
                    "snap_eligible_change": 0.0,
                    "medicaid_eligible_change": 0.0,
                })
                # Employment changes hit lower quintiles harder
                emp_weights = [0.30, 0.25, 0.20, 0.15, 0.10]
                county_pop = _county_population(counties, county_id)
                # Convert employment fraction to approximate income impact
                income_effect = county_employment_impact * gdp / max(county_pop, 1) * 0.1
                for q in range(5):
                    existing["income_delta_by_quintile"][q] += (
                        income_effect * emp_weights[q] * 5
                    )

                # Sector contraction pushes people into benefit eligibility
                if county_employment_impact < 0:
                    existing["snap_eligible_change"] += (
                        abs(county_employment_impact) * 0.3
                    )
                    existing["medicaid_eligible_change"] += (
                        abs(county_employment_impact) * 0.2
                    )

                county_deltas[county_id] = existing

        # ── 2. Apply fiscal bill events ──────────────────────────────
        for event in fiscal_events:
            bill_type = event.get("bill_type", "spending")
            amount_gdp_pct = event.get("amount_gdp_pct", 0.0)
            affected_counties = event.get("affected_counties", list(counties.keys()))
            sector = event.get("sector")

            weights = BILL_TYPE_WEIGHTS.get(bill_type, SPENDING_WEIGHTS)
            total_amount = amount_gdp_pct * gdp

            for county_id in affected_counties:
                county_pop = _county_population(counties, county_id)
                per_capita = total_amount / max(county_pop, 1)

                income_by_quintile = [per_capita * w * 5 for w in weights]

                existing = county_deltas.get(county_id, {
                    "income_delta_by_quintile": [0.0] * 5,
                    "snap_eligible_change": 0.0,
                    "medicaid_eligible_change": 0.0,
                })

                for q in range(5):
                    existing["income_delta_by_quintile"][q] += income_by_quintile[q]

                # Benefit eligibility shifts based on macro conditions
                snap_change, medicaid_change = _benefit_eligibility_shift(
                    unemployment, income_by_quintile, bill_type,
                )
                existing["snap_eligible_change"] += snap_change
                existing["medicaid_eligible_change"] += medicaid_change

                county_deltas[county_id] = existing

            # Narrative seed
            narrative_seeds.extend(
                _generate_narratives(bill_type, amount_gdp_pct, weights,
                                     gdp, affected_counties, sector)
            )

        # ── 3. Narrative seeds for sector-driven county impacts ──────
        if has_sector_movement:
            contracting = [
                name for name, sd in sector_deltas.items()
                if sd.get("employment_delta", 0) < -0.01
            ]
            expanding = [
                name for name, sd in sector_deltas.items()
                if sd.get("employment_delta", 0) > 0.01
            ]
            if contracting:
                narrative_seeds.append(
                    f"Job losses in {', '.join(contracting)} "
                    f"affecting household incomes"
                )
            if expanding:
                narrative_seeds.append(
                    f"Hiring in {', '.join(expanding)} "
                    f"boosting household incomes"
                )

        # Merge into delta
        for cid, cd in county_deltas.items():
            delta.setdefault("county_deltas", {})[cid] = cd

        delta.setdefault("narrative_seeds", []).extend(narrative_seeds)

        # Cache
        self._cache_key = cache_key
        self._cached_results = {
            "county_deltas": county_deltas,
            "narrative_seeds": narrative_seeds,
        }

        return delta


# ----------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------

def _event_type(event: dict) -> str:
    """Extract event type, handling both flat and serde-tagged formats."""
    if "type" in event:
        return event["type"]
    if len(event) == 1:
        return next(iter(event))
    return ""


def _county_population(counties: dict, county_id: str) -> int:
    """Return population for a county, defaulting to 100 000."""
    county = counties.get(county_id, {})
    return county.get("population", 100_000)


def _build_cache_key(fiscal_events: list[dict], macro: dict) -> str:
    """Build a simple cache key from events + rounded macro values."""
    event_part = str(sorted(
        (e.get("bill_type", ""), e.get("amount_gdp_pct", 0)) for e in fiscal_events
    ))
    macro_part = (
        f"{macro.get('unemployment', 0):.3f}|"
        f"{macro.get('gdp_growth', 0):.3f}|"
        f"{macro.get('inflation', 0):.3f}"
    )
    return f"{event_part}|{macro_part}"


def _benefit_eligibility_shift(
    unemployment: float,
    income_by_quintile: list[float],
    bill_type: str,
) -> tuple[float, float]:
    """Estimate change in SNAP/Medicaid eligible fraction (as pct-point delta).

    Returns (snap_change, medicaid_change) — positive means more people eligible.
    """
    snap_change = 0.0
    medicaid_change = 0.0

    # High unemployment pushes more people into benefit eligibility
    if unemployment > 0.06:
        excess = unemployment - 0.06
        snap_change += excess * 0.5  # 1pp unemployment → +0.5pp SNAP eligible
        medicaid_change += excess * 0.3

    # If bottom quintile income drops, more become eligible
    if income_by_quintile[0] < 0:
        snap_change += abs(income_by_quintile[0]) / SNAP_THRESHOLD * 0.1
        medicaid_change += abs(income_by_quintile[0]) / MEDICAID_THRESHOLD * 0.08

    # If bottom quintile income rises substantially, fewer are eligible
    # Only applies for non-spending bills (spending bills expand programs)
    if income_by_quintile[0] > 0 and bill_type not in ("spending",):
        snap_change -= income_by_quintile[0] / SNAP_THRESHOLD * 0.01
        medicaid_change -= income_by_quintile[0] / MEDICAID_THRESHOLD * 0.008

    return snap_change, medicaid_change


def _merge_cached(delta: dict, cached: dict) -> None:
    """Merge cached results into the live delta."""
    for cid, cd in cached.get("county_deltas", {}).items():
        delta.setdefault("county_deltas", {})[cid] = cd
    delta.setdefault("narrative_seeds", []).extend(
        cached.get("narrative_seeds", [])
    )


def _generate_narratives(
    bill_type: str,
    amount_gdp_pct: float,
    weights: list[float],
    gdp: float,
    affected_counties: list[str],
    sector: str | None,
) -> list[str]:
    """Create human-readable narrative seeds for the event."""
    seeds: list[str] = []
    total = amount_gdp_pct * gdp
    bottom_share = total * weights[0]
    top_share = total * weights[4]
    monthly_bottom = bottom_share / 12
    monthly_top = top_share / 12

    region = affected_counties[0] if len(affected_counties) == 1 else "affected areas"

    if bill_type == "tax_cut":
        seeds.append(
            f"Tax cut: top quintile in {region} gains ~${monthly_top:,.0f}/mo; "
            f"bottom quintile gains ~${monthly_bottom:,.0f}/mo"
        )
    elif bill_type == "spending":
        seeds.append(
            f"Spending increase: bottom quintile in {region} gains "
            f"~${monthly_bottom:,.0f}/mo from expanded benefits"
        )
    elif bill_type == "mandate":
        sector_label = sector or "affected sector"
        seeds.append(
            f"New mandate impacts {sector_label} employment in {region}"
        )
    elif bill_type == "regulation":
        sector_label = sector or "regulated sector"
        seeds.append(
            f"Regulation increases costs in {sector_label}; "
            f"top quintile absorbs ~${monthly_top:,.0f}/mo"
        )

    return seeds
