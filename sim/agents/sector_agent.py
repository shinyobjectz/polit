"""SectorAgent — Mesa 3.x agent representing one economic sector.

Each agent tracks output, employment, price level, demand, and supply
for a single sector (Energy, Tech, Pharma, etc.). Macro conditions
flow in from the simulation host; the agent adjusts its internal state
each tick using simple lag-based dynamics.
"""

from __future__ import annotations

import mesa


class SectorAgent(mesa.Agent):
    """A single economic sector in the Mesa model.

    Attributes:
        sector_name: Human-readable sector label (e.g. ``"Energy"``).
        output: Production level relative to baseline (1.0 = normal).
        employment: Employment level relative to baseline.
        price_level: Price level relative to baseline.
        demand: Current demand pressure (1.0 = neutral).
        supply: Current supply capacity (1.0 = neutral).
        macro_gdp_growth: GDP growth rate injected by the layer each tick.
        macro_fed_rate: Federal funds rate injected by the layer each tick.
    """

    def __init__(
        self,
        model: mesa.Model,
        *,
        sector_name: str,
        output: float = 1.0,
        employment: float = 1.0,
        price_level: float = 1.0,
        demand: float = 1.0,
        supply: float = 1.0,
    ) -> None:
        super().__init__(model)
        self.sector_name = sector_name
        self.output = output
        self.employment = employment
        self.price_level = price_level
        self.demand = demand
        self.supply = supply

        # Macro conditions — set externally before each step.
        self.macro_gdp_growth: float = 0.02
        self.macro_fed_rate: float = 0.05

    def step(self) -> None:
        """Advance the sector by one tick."""

        # Demand adjusts based on macro conditions
        demand_pressure = self.macro_gdp_growth * 2.0 - self.macro_fed_rate * 0.5
        self.demand = max(0.5, min(1.5, 1.0 + demand_pressure))

        # Output tracks demand with lag (firms adjust slowly)
        output_gap = self.demand - self.output
        self.output += output_gap * 0.3  # 30% adjustment per tick

        # Employment tracks output with smaller lag
        employment_gap = self.output - self.employment
        self.employment += employment_gap * 0.2

        # Prices rise when demand > supply, fall when supply > demand
        price_pressure = (self.demand - self.supply) * 0.1
        self.price_level = max(0.5, self.price_level + price_pressure)

        # Supply adjusts toward output over time
        supply_gap = self.output - self.supply
        self.supply += supply_gap * 0.1
