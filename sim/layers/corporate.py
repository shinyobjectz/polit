"""CorporateLayer — wraps a Mesa CorporateModel for corporate political behaviour.

Nine corporate-bloc agents (one per sector) react to FiscalBill events
by computing policy impact against their interests and producing
lobby / donate / retaliate actions per the GDD §16 reaction matrix.
"""

from __future__ import annotations

import mesa

from sim.agents.corporate_agent import CorporateAgent
from sim.layers.base import SimulationLayer

# GDD §16 sector interest lookup table.
SECTOR_PROFILES: list[dict] = [
    {
        "sector": "Energy",
        "wants": ["deregulation", "drilling", "tax_breaks"],
        "opposes": ["carbon_tax", "renewable_mandates"],
        "lobby_intensity": 0.8,
        "donation_pattern": "right",
    },
    {
        "sector": "Tech",
        "wants": ["h1b", "ip_protection", "section_230"],
        "opposes": ["data_privacy", "content_liability"],
        "lobby_intensity": 0.9,
        "donation_pattern": "mixed",
    },
    {
        "sector": "Pharma",
        "wants": ["patent_extension", "fast_fda"],
        "opposes": ["drug_negotiation", "generics"],
        "lobby_intensity": 0.9,
        "donation_pattern": "bipartisan",
    },
    {
        "sector": "Defense",
        "wants": ["military_spending", "intervention"],
        "opposes": ["defense_cuts", "diplomacy_first"],
        "lobby_intensity": 0.9,
        "donation_pattern": "bipartisan",
    },
    {
        "sector": "Finance",
        "wants": ["deregulation", "low_capital_req"],
        "opposes": ["dodd_frank", "transaction_tax"],
        "lobby_intensity": 0.9,
        "donation_pattern": "bipartisan",
    },
    {
        "sector": "Manufacturing",
        "wants": ["tariffs", "infrastructure", "tax_incentives"],
        "opposes": ["min_wage", "environmental_regs"],
        "lobby_intensity": 0.6,
        "donation_pattern": "right",
    },
    {
        "sector": "Agriculture",
        "wants": ["subsidies", "water_rights", "trade_deals"],
        "opposes": ["environmental_regs", "labor_regs"],
        "lobby_intensity": 0.6,
        "donation_pattern": "right",
    },
    {
        "sector": "Healthcare",
        "wants": ["reimbursement_rates", "liability_caps"],
        "opposes": ["single_payer", "price_transparency"],
        "lobby_intensity": 0.8,
        "donation_pattern": "bipartisan",
    },
    {
        "sector": "Retail",
        "wants": ["low_min_wage", "reduced_benefits"],
        "opposes": ["union_protections", "scheduling_regs"],
        "lobby_intensity": 0.6,
        "donation_pattern": "right",
    },
]


class CorporateModel(mesa.Model):
    """Mesa model containing one :class:`CorporateAgent` per sector."""

    def __init__(self) -> None:
        super().__init__()
        for profile in SECTOR_PROFILES:
            CorporateAgent(
                self,
                sector=profile["sector"],
                lobby_intensity=profile["lobby_intensity"],
                donation_pattern=profile["donation_pattern"],
                wants=profile["wants"],
                opposes=profile["opposes"],
            )

    def step(self) -> None:  # type: ignore[override]
        self.agents.shuffle_do("step")


class CorporateLayer(SimulationLayer):
    """Simulation layer that evaluates corporate reactions to policy events.

    Reads ``FiscalBill`` events, scores each against every sector's
    interests, generates reaction actions per the GDD §16 matrix, and
    updates sector leverage from sector output deltas.
    """

    def __init__(self) -> None:
        self._model = CorporateModel()

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    def _agent_by_sector(self, sector: str) -> CorporateAgent | None:
        for agent in self._model.agents:
            if agent.sector == sector:
                return agent
        return None

    # ------------------------------------------------------------------
    # SimulationLayer interface
    # ------------------------------------------------------------------

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        # 1. Update leverage from sector output deltas
        sector_deltas = (
            delta.get("sector_deltas")
            or world_state.get("sector_deltas", {})
        )
        for agent in self._model.agents:
            sd = sector_deltas.get(agent.sector, {})
            output_delta = sd.get("output_delta", 0.0)
            # Leverage tracks relative economic weight: stronger sectors
            # have more political clout.  Clamp to [0.5, 2.0].
            agent.leverage = max(0.5, min(2.0, 1.0 + output_delta))

        # 2. Process FiscalBill events
        corporate_actions: list[dict] = []

        for event in events:
            if event.get("type") != "FiscalBill":
                continue

            bill_type = event.get("bill_type", "")
            sector_target = event.get("sector_target", "")

            for agent in self._model.agents:
                impact = agent.compute_policy_impact(bill_type, sector_target)
                if impact == 0.0:
                    continue
                actions = agent.react(impact)
                corporate_actions.extend(actions)

        # 3. Step the Mesa model (no-op agents, but keeps framework happy)
        self._model.step()

        # 4. Write results into delta
        delta.setdefault("corporate_actions", [])
        delta["corporate_actions"].extend(corporate_actions)

        # Narrative seeds for significant corporate reactions
        delta.setdefault("narrative_seeds", [])
        for action in corporate_actions:
            if action["type"] in (
                "threaten_plant_closure",
                "legal_challenge",
                "major_donation",
            ):
                delta["narrative_seeds"].append(
                    f"{action['sector']} sector: {action['type'].replace('_', ' ')}"
                )

        return delta
