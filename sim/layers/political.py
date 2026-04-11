"""PoliticalLayer -- translates macro economics into political signals.

Wraps a Mesa model of VoterAgents that represent demographic cohorts.
Each tick the layer:
  1. Injects current macro conditions into the model.
  2. Processes political events (scandals, protests).
  3. Steps all voter agents (opinion dynamics).
  4. Aggregates results into approval, protest risk, and ideology deltas.
"""

from __future__ import annotations

import mesa

from sim.agents.voter_agent import VoterAgent
from sim.layers.base import SimulationLayer

# Default demographic groups with baseline parameters.
# (county_fips "00000" = national placeholder until county data is wired in)
DEFAULT_DEMOGRAPHICS: list[dict] = [
    {
        "demographic": "young_college",
        "population_share": 0.15,
        "ideology_score": -0.2,
        "turnout_propensity": 0.45,
        "trust_institutions": 0.55,
    },
    {
        "demographic": "young_nocollege",
        "population_share": 0.15,
        "ideology_score": 0.1,
        "turnout_propensity": 0.35,
        "trust_institutions": 0.40,
    },
    {
        "demographic": "middle_age",
        "population_share": 0.30,
        "ideology_score": 0.0,
        "turnout_propensity": 0.60,
        "trust_institutions": 0.50,
    },
    {
        "demographic": "older_working",
        "population_share": 0.20,
        "ideology_score": 0.15,
        "turnout_propensity": 0.70,
        "trust_institutions": 0.55,
    },
    {
        "demographic": "retired",
        "population_share": 0.20,
        "ideology_score": 0.2,
        "turnout_propensity": 0.75,
        "trust_institutions": 0.60,
    },
]


class PoliticalModel(mesa.Model):
    """Mesa model containing VoterAgent instances."""

    def __init__(self) -> None:
        super().__init__()
        self.macro_gdp_growth: float = 0.02
        self.macro_unemployment: float = 0.045
        self.macro_confidence: float = 100.0

        for cfg in DEFAULT_DEMOGRAPHICS:
            VoterAgent(
                self,
                county_fips="00000",
                demographic=cfg["demographic"],
                population_share=cfg["population_share"],
                ideology_score=cfg.get("ideology_score", 0.0),
                turnout_propensity=cfg.get("turnout_propensity", 0.60),
                trust_institutions=cfg.get("trust_institutions", 0.50),
            )

    def step(self) -> None:  # type: ignore[override]
        self.agents.shuffle_do("step")


def _normalize(raw: dict) -> dict:
    """Normalise a serde-tagged event to flat dict with 'type' key."""
    if "type" in raw:
        return raw
    if len(raw) == 1:
        variant = next(iter(raw))
        fields = raw[variant]
        if isinstance(fields, dict):
            return {"type": variant, **fields}
        return {"type": variant, "value": fields}
    return raw


class PoliticalLayer(SimulationLayer):
    """Simulation layer for political opinion dynamics."""

    def __init__(self) -> None:
        self._model = PoliticalModel()

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        macro = world_state.get("macro", {})

        # Inject current macro conditions (base + accumulated deltas)
        self._model.macro_gdp_growth = (
            macro.get("gdp_growth", 0.02)
            + delta.get("gdp_growth_delta", 0.0)
        )
        self._model.macro_unemployment = (
            macro.get("unemployment", 0.045)
            + delta.get("unemployment_delta", 0.0)
        )
        self._model.macro_confidence = (
            macro.get("consumer_confidence", 100)
            + delta.get("consumer_confidence_delta", 0.0)
        )

        # Process scandal events before stepping (trust affects turnout calc)
        for raw_event in events:
            event = _normalize(raw_event)
            etype = event.get("type", "")

            if etype == "Scandal":
                severity = event.get("severity", 0)
                for agent in self._model.agents:
                    agent.trust_institutions -= severity * 0.05
                    agent.trust_institutions = max(0.0, agent.trust_institutions)

        # Step all voter agents (recalculates turnout from anxiety + trust)
        self._model.step()

        # Apply protest effects after stepping so they aren't overwritten
        for raw_event in events:
            event = _normalize(raw_event)
            if event.get("type", "") == "Protest":
                scale = event.get("scale", 0)
                for agent in self._model.agents:
                    agent.turnout_propensity = min(
                        0.95, agent.turnout_propensity + scale * 0.02
                    )

        # -- Aggregate political deltas ----------------------------------

        agents = list(self._model.agents)
        if not agents:
            return delta

        # Weighted averages
        total_share = sum(a.population_share for a in agents)
        if total_share == 0:
            total_share = 1.0

        avg_anxiety = sum(
            a.economic_anxiety * a.population_share for a in agents
        ) / total_share

        avg_trust = sum(
            a.trust_institutions * a.population_share for a in agents
        ) / total_share

        # Approval ratings: anxiety hurts incumbent
        delta["approval_president_delta"] = (
            delta.get("approval_president_delta", 0.0) - avg_anxiety * 5.0
        )
        delta["approval_congress_delta"] = (
            delta.get("approval_congress_delta", 0.0) - avg_anxiety * 3.0
        )

        # Protest risk: high anxiety + low trust
        protest_risk = max(0.0, avg_anxiety * (1.0 - avg_trust))
        if protest_risk > 0.1:
            delta.setdefault("protest_risk_by_region", {})["national"] = (
                protest_risk
            )
            delta.setdefault("narrative_seeds", []).append(
                f"Social unrest risk elevated ({protest_risk:.0%})"
            )

        # Ideology shifts
        ideology_shifts = delta.setdefault("voter_ideology_shifts", [])
        for agent in agents:
            if abs(agent.ideology_score) > 0.01:
                ideology_shifts.append({
                    "demographic_group": (
                        f"{agent.county_fips}_{agent.demographic}"
                    ),
                    "direction": 1.0 if agent.ideology_score > 0 else -1.0,
                    "magnitude": abs(agent.ideology_score),
                })

        return delta
