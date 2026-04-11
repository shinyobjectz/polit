"""CorporateAgent — Mesa 3.x agent representing one sector's corporate bloc.

Each agent tracks a sector's political interests (wants/opposes),
lobby intensity, donation patterns, and leverage. When policy events
occur, the agent computes reactions per the GDD section 16 reaction
matrix, producing lobby/donate/retaliate actions for downstream
consumption by the Rust NPC layer.
"""

from __future__ import annotations

import mesa


class CorporateAgent(mesa.Agent):
    """A corporate bloc for a single economic sector.

    Attributes:
        sector: Sector name (e.g. ``"Energy"``).
        lobby_intensity: Base lobbying aggressiveness, 0-1.
        donation_pattern: ``"right"``, ``"left"``, ``"bipartisan"``, or ``"mixed"``.
        wants: Policy types this sector favours.
        opposes: Policy types this sector opposes.
        leverage: Multiplier derived from sector economic weight.
    """

    def __init__(
        self,
        model: mesa.Model,
        *,
        sector: str,
        lobby_intensity: float,
        donation_pattern: str,
        wants: list[str],
        opposes: list[str],
    ) -> None:
        super().__init__(model)
        self.sector = sector
        self.lobby_intensity = lobby_intensity
        self.donation_pattern = donation_pattern
        self.wants = list(wants)
        self.opposes = list(opposes)
        self.leverage = 1.0

    # ------------------------------------------------------------------
    # Policy impact scoring
    # ------------------------------------------------------------------

    def compute_policy_impact(self, bill_type: str, sector_target: str) -> float:
        """Compute how a policy affects this sector's interests.

        Returns a value in ``[-1.0, 1.0]``.
        """
        impact = 0.0
        if bill_type in self.wants:
            impact = 0.5
        elif bill_type in self.opposes:
            impact = -0.5

        # Bills targeting this sector specifically have double effect.
        if sector_target == self.sector:
            impact *= 2.0

        return max(-1.0, min(1.0, impact))

    # ------------------------------------------------------------------
    # Reaction matrix (GDD §16)
    # ------------------------------------------------------------------

    def react(self, impact: float) -> list[dict]:
        """Return corporate actions based on net policy impact.

        The reaction intensity is ``impact × lobby_intensity × leverage``.
        Actions follow the GDD reaction matrix thresholds.
        """
        reaction_intensity = impact * self.lobby_intensity * self.leverage

        actions: list[dict] = []

        if reaction_intensity >= 0.3:
            actions.append({
                "type": "major_donation",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
            actions.append({
                "type": "endorsement",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
        elif reaction_intensity >= 0.1:
            actions.append({
                "type": "quiet_donation",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
        elif reaction_intensity <= -0.5:
            actions.append({
                "type": "threaten_plant_closure",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
            actions.append({
                "type": "legal_challenge",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
        elif reaction_intensity <= -0.3:
            actions.append({
                "type": "attack_ads",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
            actions.append({
                "type": "fund_opposition",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
        elif reaction_intensity <= -0.1:
            actions.append({
                "type": "lobby_against",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })
            actions.append({
                "type": "shift_donations",
                "sector": self.sector,
                "intensity": reaction_intensity,
            })

        return actions

    def step(self) -> None:
        """No-op — reactions are driven externally by the layer."""
