"""Simplified macro economy layer inspired by PyFRB/US.

Models GDP, inflation, unemployment, interest rates using
simplified Keynesian/monetarist relationships.  Runs every tick
(weekly) but phases fiscal shocks in gradually so the game world
feels smooth rather than jumpy.
"""

from __future__ import annotations

import math
from typing import Any

from .base import SimulationLayer

# ── Equilibrium constants ────────────────────────────────────────────
TREND_GDP_GROWTH = 0.02  # 2 % annual
NATURAL_UNEMPLOYMENT = 0.045  # 4.5 %
TARGET_INFLATION = 0.02  # 2 %
NEUTRAL_RATE = 0.025  # 2.5 % neutral fed-funds rate

# ── Mean-reversion speeds (per tick ≈ per week) ──────────────────────
# ~52 ticks/year.  We want half-life of ~1 year → λ = ln(2)/52 ≈ 0.013
MEAN_REVERT_GDP = 0.013
MEAN_REVERT_UNEMP = 0.010
MEAN_REVERT_INFL = 0.008
MEAN_REVERT_CONFIDENCE = 0.015

# ── Fiscal multipliers ───────────────────────────────────────────────
SPENDING_MULTIPLIER = 1.5
TAX_CUT_MULTIPLIER = 0.7

# ── Phase-in profile (fraction of total impulse realised by week) ───
# Effects phase in over ~20 weeks.  We store an impulse and decay it.
IMPULSE_DECAY = 0.92  # per-tick retention of remaining impulse

# ── Relationship constants ───────────────────────────────────────────
PHILLIPS_COEFF = -0.3 / 13  # per quarter → per tick (13 ticks/quarter)
OKUN_COEFF = -0.5
TAYLOR_INFLATION = 1.5
TAYLOR_OUTPUT = 0.5
CONFIDENCE_GDP_COEFF = 50.0
CONFIDENCE_UNEMP_COEFF = -200.0

TICKS_PER_YEAR = 52


def _annualised_to_weekly(annual_rate: float) -> float:
    """Convert an annual growth rate to a weekly delta."""
    return annual_rate / TICKS_PER_YEAR


class _FiscalImpulse:
    """Tracks a single fiscal shock as it phases in over time."""

    __slots__ = ("gdp_effect", "inflation_nudge", "remaining")

    def __init__(self, gdp_effect: float, inflation_nudge: float = 0.0):
        self.gdp_effect = gdp_effect
        self.inflation_nudge = inflation_nudge
        self.remaining = 1.0  # fraction not yet realised

    def tick(self) -> tuple[float, float]:
        """Return (gdp_delta, inflation_delta) for this tick."""
        realised = self.remaining * (1.0 - IMPULSE_DECAY)
        self.remaining *= IMPULSE_DECAY
        return (
            self.gdp_effect * realised,
            self.inflation_nudge * realised,
        )

    @property
    def exhausted(self) -> bool:
        return self.remaining < 1e-6


class MacroEconomyLayer(SimulationLayer):
    """Simplified macro model inspired by PyFRB/US.

    Models GDP, inflation, unemployment, interest rates using
    simplified Keynesian/monetarist relationships.
    """

    def __init__(self) -> None:
        self._impulses: list[_FiscalImpulse] = []

    # ── public interface ─────────────────────────────────────────────

    def step(
        self,
        world_state: dict[str, Any],
        events: list[dict[str, Any]],
        delta: dict[str, Any],
    ) -> dict[str, Any]:
        macro = world_state.get("macro", {})

        # Current levels (from world_state or sensible defaults)
        gdp_growth = macro.get("gdp_growth", TREND_GDP_GROWTH)
        unemployment = macro.get("unemployment", NATURAL_UNEMPLOYMENT)
        inflation = macro.get("inflation", TARGET_INFLATION)
        confidence = macro.get("consumer_confidence", 100.0)

        # 0. Ingest new shocks from events
        self._ingest_events(events)

        # 1. Fiscal / monetary impulse contributions
        impulse_gdp, impulse_infl = self._sum_impulses()

        # 2. GDP delta: trend + impulse + mean-reversion
        gdp_delta = self._compute_gdp_delta(gdp_growth, impulse_gdp)

        # 3. Inflation: Phillips curve + impulse + mean-reversion
        inflation_delta = self._compute_inflation_delta(
            inflation, unemployment, impulse_infl
        )

        # 4. Unemployment: Okun's law + mean-reversion
        unemployment_delta = self._compute_unemployment_delta(
            unemployment, gdp_growth, gdp_delta
        )

        # 5. Fed-funds rate via Taylor rule
        fed_funds = self._compute_fed_funds(inflation, gdp_growth)

        # 6. Consumer confidence
        confidence_delta = self._compute_confidence_delta(
            confidence, gdp_delta, unemployment_delta
        )

        # Write into delta dict
        delta["gdp_growth_delta"] += gdp_delta
        delta["inflation_delta"] += inflation_delta
        delta["unemployment_delta"] += unemployment_delta
        delta["fed_funds_rate"] = fed_funds
        delta["consumer_confidence_delta"] += confidence_delta

        # Narrative seeds for notable movements
        self._emit_narratives(delta, gdp_delta, inflation_delta, unemployment_delta)

        return delta

    # ── private helpers ──────────────────────────────────────────────

    def _ingest_events(self, events: list[dict[str, Any]]) -> None:
        for raw_event in events:
            event = _normalize(raw_event)
            etype = event.get("type", "")

            if etype == "FiscalBill":
                spending = event.get("spending_gdp_pct", 0.0)
                tax_cut = event.get("tax_cut_gdp_pct", 0.0)
                gdp_fx = spending * SPENDING_MULTIPLIER + tax_cut * TAX_CUT_MULTIPLIER
                # Spending is mildly inflationary
                infl_fx = spending * 0.3 + tax_cut * 0.1
                self._impulses.append(_FiscalImpulse(gdp_fx, infl_fx))

            elif etype == "MonetaryPolicy":
                rate_delta = event.get("fed_funds_delta", 0.0)
                # Rate hike → GDP drag, inflation down
                gdp_fx = -rate_delta * 0.8
                infl_fx = -rate_delta * 0.4
                self._impulses.append(_FiscalImpulse(gdp_fx, infl_fx))

            elif etype == "Tariff":
                tariff_pct = event.get("tariff_pct", 0.10)
                # Tariffs: supply shock — GDP drag + inflation up
                gdp_fx = -tariff_pct * 0.5
                infl_fx = tariff_pct * 0.6
                self._impulses.append(_FiscalImpulse(gdp_fx, infl_fx))

            elif etype == "EconomyShock":
                severity = event.get("severity", 0.0)
                gdp_fx = -severity * 0.02
                infl_fx = severity * 0.01
                self._impulses.append(_FiscalImpulse(gdp_fx, infl_fx))

    def _sum_impulses(self) -> tuple[float, float]:
        """Tick all active impulses and return combined (gdp, infl) deltas."""
        total_gdp = 0.0
        total_infl = 0.0
        alive: list[_FiscalImpulse] = []
        for imp in self._impulses:
            g, i = imp.tick()
            total_gdp += g
            total_infl += i
            if not imp.exhausted:
                alive.append(imp)
        self._impulses = alive
        return total_gdp, total_infl

    def _compute_gdp_delta(
        self, current_growth: float, impulse: float
    ) -> float:
        weekly_trend = _annualised_to_weekly(TREND_GDP_GROWTH)
        weekly_current = _annualised_to_weekly(current_growth)
        mean_revert = MEAN_REVERT_GDP * (weekly_trend - weekly_current)
        return mean_revert + impulse

    def _compute_inflation_delta(
        self,
        current_inflation: float,
        unemployment: float,
        impulse: float,
    ) -> float:
        # Phillips curve component (weekly)
        phillips = PHILLIPS_COEFF * (unemployment - NATURAL_UNEMPLOYMENT)
        # Mean reversion
        mean_revert = MEAN_REVERT_INFL * (TARGET_INFLATION - current_inflation)
        return phillips + mean_revert + impulse

    def _compute_unemployment_delta(
        self,
        current_unemp: float,
        gdp_growth: float,
        gdp_delta: float,
    ) -> float:
        # Okun's law: GDP growth above trend lowers unemployment
        effective_growth = gdp_growth + gdp_delta
        weekly_trend = _annualised_to_weekly(TREND_GDP_GROWTH)
        weekly_growth = _annualised_to_weekly(effective_growth)
        okun = OKUN_COEFF * (weekly_growth - weekly_trend)
        # Mean reversion
        mean_revert = MEAN_REVERT_UNEMP * (NATURAL_UNEMPLOYMENT - current_unemp)
        return okun + mean_revert

    def _compute_fed_funds(
        self, inflation: float, gdp_growth: float
    ) -> float:
        rate = (
            NEUTRAL_RATE
            + TAYLOR_INFLATION * (inflation - TARGET_INFLATION)
            + TAYLOR_OUTPUT * (gdp_growth - TREND_GDP_GROWTH)
        )
        # Floor at 0 (zero lower bound)
        return max(0.0, rate)

    def _compute_confidence_delta(
        self,
        current_confidence: float,
        gdp_delta: float,
        unemployment_delta: float,
    ) -> float:
        raw = (
            CONFIDENCE_GDP_COEFF * gdp_delta
            + CONFIDENCE_UNEMP_COEFF * unemployment_delta
        )
        # Mean-revert toward 100
        mean_revert = MEAN_REVERT_CONFIDENCE * (100.0 - current_confidence)
        return raw + mean_revert

    def _emit_narratives(
        self,
        delta: dict[str, Any],
        gdp_delta: float,
        inflation_delta: float,
        unemployment_delta: float,
    ) -> None:
        if abs(gdp_delta) > 0.005:
            direction = "growing" if gdp_delta > 0 else "contracting"
            delta["narrative_seeds"].append(
                f"Economy {direction} at {abs(gdp_delta) * 100:.1f}% rate"
            )
        if abs(inflation_delta) > 0.003:
            direction = "rising" if inflation_delta > 0 else "falling"
            delta["narrative_seeds"].append(
                f"Inflation {direction} ({abs(inflation_delta) * 100:.2f}%)"
            )
        if abs(unemployment_delta) > 0.002:
            direction = "rising" if unemployment_delta > 0 else "falling"
            delta["narrative_seeds"].append(
                f"Unemployment {direction} ({abs(unemployment_delta) * 100:.2f}%)"
            )


def _normalize(raw: dict) -> dict:
    """Normalise serde-tagged enum dicts to flat {type: ...} form."""
    if "type" in raw:
        return raw
    if len(raw) == 1:
        variant = next(iter(raw))
        fields = raw[variant]
        if isinstance(fields, dict):
            return {"type": variant, **fields}
        return {"type": variant, "value": fields}
    return raw
