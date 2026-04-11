"""CountryAgent — Mesa 3.x agent representing a foreign power.

Each agent tracks military, economic, diplomatic, nuclear, stability,
and trade attributes for a single country.  Alignment drifts slowly
toward neutral between events; stability mean-reverts toward 50.
"""

from __future__ import annotations

import mesa


class CountryAgent(mesa.Agent):
    """A single foreign power in the Mesa model.

    Attributes:
        name: Country / entity name (e.g. ``"China"``).
        tier: Importance tier (1 = major, 2 = regional, 3 = minor).
        alignment: Relationship to the US, -1 (enemy) to +1 (ally).
        military: Military capability score 0–100.
        economic: Economic capability score 0–100.
        diplomatic: Diplomatic capability score 0–100.
        nuclear: Nuclear capability score 0–100 (0 = no nukes).
        stability: Internal stability score 0–100.
        trade_with_us: Bilateral trade dict ``{"imports": float, "exports": float}``
            in billions USD.
    """

    def __init__(
        self,
        model: mesa.Model,
        *,
        name: str,
        tier: int,
        alignment: float,
        military: float,
        economic: float,
        diplomatic: float,
        nuclear: float,
        stability: float,
        trade_with_us: dict[str, float],
    ) -> None:
        super().__init__(model)
        self.name = name
        self.tier = tier
        self.alignment = alignment
        self.military = military
        self.economic = economic
        self.diplomatic = diplomatic
        self.nuclear = nuclear
        self.stability = stability
        self.trade_with_us = dict(trade_with_us)  # defensive copy

    def step(self) -> None:
        """Advance one tick — slow drift between events."""
        # Alignment drifts toward 0 (neutral) very slowly
        self.alignment *= 0.999
        # Stability mean-reverts toward 50
        self.stability += (50 - self.stability) * 0.01


def compute_migration_pressure(country: CountryAgent) -> float:
    """Gravity-model migration pressure from *country* toward the US.

    Higher instability pushes more migrants; US economic pull is fixed
    at 0.7 baseline; enforcement barrier at 0.5 baseline.
    """
    push = max(0.0, (50 - country.stability) / 50)
    pull = 0.7  # US economic opportunity (baseline)
    barrier = 0.5  # immigration enforcement (baseline)
    return (push * pull) / max(0.1, barrier)
