"""MarketLayer — wraps a Mesa MarketModel for financial market simulation.

Thirteen agents (9 sector indices, 3 commodities, 1 bond) react to macro
conditions, sector output, and event shocks each tick, producing market
prices that feed back into household wealth effects and narrative flavor.
"""

from __future__ import annotations

import mesa

from sim.agents.market_agent import BondAgent, CommodityAgent, SectorIndexAgent
from sim.layers.base import SimulationLayer

SECTOR_NAMES: list[str] = [
    "Energy",
    "Tech",
    "Pharma",
    "Defense",
    "Finance",
    "Manufacturing",
    "Agriculture",
    "Healthcare",
    "Retail",
]

COMMODITY_NAMES: list[str] = ["oil", "food", "metals"]


class MarketModel(mesa.Model):
    """Mesa model containing sector index, commodity, and bond agents."""

    def __init__(self) -> None:
        super().__init__()

        # Macro conditions — set externally before each step.
        self.macro_gdp: float = 0.02
        self.macro_fed_rate: float = 0.025
        self.macro_confidence: float = 100.0
        self.macro_debt_to_gdp: float = 1.0

        # Sector output levels — populated from sector_deltas each tick.
        self.sector_outputs: dict[str, float] = {}

        # Create agents
        for name in SECTOR_NAMES:
            SectorIndexAgent(self, sector_name=name)

        for commodity in COMMODITY_NAMES:
            CommodityAgent(self, commodity_name=commodity)

        BondAgent(self)

    def step(self) -> None:  # type: ignore[override]
        self.agents.shuffle_do("step")


class MarketLayer(SimulationLayer):
    """Simulation layer that drives financial markets each tick.

    Reads macro conditions and sector deltas from *world_state* / *delta*,
    applies event shocks, steps the Mesa model, and writes market prices
    + narrative seeds into *delta*.
    """

    def __init__(self) -> None:
        self._model = MarketModel()

    # ------------------------------------------------------------------
    # SimulationLayer interface
    # ------------------------------------------------------------------

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        macro = world_state.get("macro", {})

        # 1. Read macro conditions
        self._model.macro_gdp = (
            macro.get("gdp_growth", 0.02) + delta.get("gdp_growth_delta", 0.0)
        )
        self._model.macro_fed_rate = (
            delta.get("fed_funds_rate") or macro.get("fed_funds_rate", 0.025)
        )
        self._model.macro_confidence = macro.get("consumer_confidence", 100.0)
        self._model.macro_debt_to_gdp = macro.get("debt_to_gdp", 1.0)

        # Read sector outputs from sector_deltas (produced by SectorLayer)
        sector_deltas = delta.get("sector_deltas", {})
        self._model.sector_outputs = {
            name: sd.get("output_delta", 0.0)
            for name, sd in sector_deltas.items()
        }

        # 2. Snapshot prices before stepping (for narrative seeds)
        prev_prices: dict[str, float] = {}
        for agent in self._model.agents:
            if isinstance(agent, SectorIndexAgent):
                prev_prices[agent.sector_name] = agent.price

        # Apply shock events to relevant agents
        for event in events:
            etype = event.get("type", "")
            if etype == "SectorShock":
                sector = event.get("sector", "")
                severity = event.get("severity", 0.0)
                for agent in self._model.agents:
                    if (
                        isinstance(agent, SectorIndexAgent)
                        and agent.sector_name == sector
                    ):
                        agent.price -= severity * 10.0
                        agent.volatility += severity * 0.05
            elif etype == "Sanction":
                severity = event.get("severity", 0.0)
                for agent in self._model.agents:
                    if isinstance(agent, CommodityAgent):
                        agent.price += severity * 5.0

        # 3. Step the model
        self._model.step()

        # 4. Output market prices
        sector_indices: dict[str, float] = {}
        oil_price: float = 80.0
        bond_yield: float = 0.04

        for agent in self._model.agents:
            if isinstance(agent, SectorIndexAgent):
                sector_indices[agent.sector_name] = agent.price
            elif isinstance(agent, CommodityAgent):
                if agent.commodity_name == "oil":
                    oil_price = agent.price
            elif isinstance(agent, BondAgent):
                bond_yield = agent.yield_rate

        delta["sector_indices"] = sector_indices
        delta["oil_price"] = oil_price
        delta["bond_yield_10yr"] = bond_yield

        # Collect all commodity prices
        commodity_prices: dict[str, float] = {}
        for agent in self._model.agents:
            if isinstance(agent, CommodityAgent):
                commodity_prices[agent.commodity_name] = agent.price
        delta["commodity_prices"] = commodity_prices

        # 5. Generate narrative seeds for significant moves (>3% in a tick)
        delta.setdefault("narrative_seeds", [])
        for agent in self._model.agents:
            if isinstance(agent, SectorIndexAgent):
                prev = prev_prices.get(agent.sector_name, 100.0)
                if prev > 0:
                    pct_change = (agent.price - prev) / prev
                    if abs(pct_change) > 0.03:
                        direction = "surged" if pct_change > 0 else "dropped"
                        delta["narrative_seeds"].append(
                            f"{agent.sector_name} index {direction}"
                            f" {abs(pct_change) * 100:.1f}%"
                        )

        return delta
