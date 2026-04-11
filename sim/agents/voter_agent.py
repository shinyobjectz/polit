"""VoterAgent -- Mesa 3.x agent representing a demographic voter group.

Each agent represents a demographic cohort within a county (or nationally
when county-level population data is not yet wired in). The agent tracks
ideology, partisanship, turnout propensity, institutional trust, and
economic anxiety. These evolve each tick based on macro conditions injected
by the PoliticalLayer.
"""

from __future__ import annotations

import mesa


class VoterAgent(mesa.Agent):
    """A demographic voter group in the Mesa model.

    Attributes:
        county_fips: FIPS code for the county this group belongs to.
        demographic: Label such as ``"young_college"`` or ``"retired"``.
        population_share: Fraction of the total population this group
            represents (0-1).
        ideology_score: Position on the left-right spectrum (-1 to +1).
        partisanship: Dict of party affiliation probabilities.
        turnout_propensity: Likelihood of voting (0.2-0.95).
        trust_institutions: Trust in government institutions (0-1).
        economic_anxiety: Derived from macro conditions (0-1).
    """

    def __init__(
        self,
        model: mesa.Model,
        county_fips: str,
        demographic: str,
        population_share: float,
        *,
        ideology_score: float = 0.0,
        turnout_propensity: float = 0.60,
        trust_institutions: float = 0.50,
    ) -> None:
        super().__init__(model)
        self.county_fips = county_fips
        self.demographic = demographic
        self.population_share = population_share

        # Opinion state
        self.ideology_score = ideology_score
        self.partisanship: dict[str, float] = {
            "dem": 0.33,
            "rep": 0.33,
            "ind": 0.34,
        }
        self.turnout_propensity = turnout_propensity
        self.trust_institutions = trust_institutions
        self.economic_anxiety: float = 0.0

    def step(self) -> None:
        """Advance the voter group by one tick."""

        # 1. Economic anxiety from macro conditions
        #    High unemployment + low confidence -> high anxiety
        gdp: float = getattr(self.model, "macro_gdp_growth", 0.02)
        unemp: float = getattr(self.model, "macro_unemployment", 0.045)
        self.economic_anxiety = max(
            0.0,
            min(1.0, (unemp - 0.04) * 5 + (0.02 - gdp) * 10),
        )

        # 2. Ideology drift from economic conditions
        #    Economic hardship -> anti-incumbent drift (simplified)
        anxiety_drift = self.economic_anxiety * 0.01  # small per-tick drift
        self.ideology_score += anxiety_drift * 0.5  # muted effect
        self.ideology_score = max(-1.0, min(1.0, self.ideology_score))

        # 3. Trust erosion from scandals is handled externally by the layer

        # 4. Turnout propensity
        #    High anxiety + high trust = higher turnout (motivated)
        #    High anxiety + low trust  = lower turnout (disengaged)
        base_turnout = 0.60
        anxiety_effect = (
            self.economic_anxiety * (self.trust_institutions - 0.5) * 0.2
        )
        self.turnout_propensity = max(
            0.2, min(0.95, base_turnout + anxiety_effect)
        )
