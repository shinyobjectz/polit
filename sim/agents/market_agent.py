"""MarketAgent — Mesa 3.x agents for financial market simulation.

Three agent types model lightweight financial markets (ABIDES-inspired,
not order-book level):

- **SectorIndexAgent**: one per sector (9 total), tracks sector output +
  macro sentiment with GARCH-lite volatility clustering.
- **CommodityAgent**: oil / food / metals (3 total), driven by supply/demand
  and geopolitical events.
- **BondAgent**: 10-year Treasury (1 total), tracks fed funds rate + risk
  premium from debt/instability.
"""

from __future__ import annotations

import random

import mesa


class SectorIndexAgent(mesa.Agent):
    """A sector equity index that tracks fundamentals with noisy mean-reversion.

    Attributes:
        sector_name: Sector this index represents (e.g. ``"Tech"``).
        price: Current index level (starts at 100).
        fundamental: Fair value derived from sector output.
        volatility: Current volatility level (GARCH-lite).
    """

    def __init__(
        self,
        model: mesa.Model,
        *,
        sector_name: str,
    ) -> None:
        super().__init__(model)
        self.sector_name = sector_name
        self.price: float = 100.0
        self.fundamental: float = 100.0
        self.volatility: float = 0.02

    def step(self) -> None:
        """Advance one tick: drift toward fundamental, apply sentiment + noise."""
        # Fundamental tracks sector output (from sector_deltas in delta)
        sector_output = self.model.sector_outputs.get(self.sector_name, 0.0)
        self.fundamental = 100.0 * (1.0 + sector_output)

        # Price drifts toward fundamental (10% adjustment per tick)
        gap = self.fundamental - self.price
        self.price += gap * 0.1

        # Sentiment effect (confidence above/below 100)
        sentiment = (self.model.macro_confidence - 100.0) * 0.01
        self.price += sentiment

        # Volatility clustering (GARCH-lite)
        noise = random.gauss(0, self.volatility)
        self.price += noise
        self.volatility = max(0.01, self.volatility * 0.95 + abs(noise) * 0.1)

        # Floor — indices can't go below 10
        self.price = max(10.0, self.price)


class CommodityAgent(mesa.Agent):
    """A commodity price agent for oil, food, or metals.

    Attributes:
        commodity_name: One of ``"oil"``, ``"food"``, ``"metals"``.
        price: Current spot price.
        base_price: Long-run equilibrium price for mean reversion.
    """

    BASE_PRICES: dict[str, float] = {
        "oil": 80.0,
        "food": 100.0,
        "metals": 100.0,
    }

    def __init__(
        self,
        model: mesa.Model,
        *,
        commodity_name: str,
    ) -> None:
        super().__init__(model)
        self.commodity_name = commodity_name
        self.base_price: float = self.BASE_PRICES[commodity_name]
        self.price: float = self.base_price

    def step(self) -> None:
        """Advance one tick: mean-revert + GDP demand effect."""
        # Mean revert toward base (5% per tick)
        gap = self.base_price - self.price
        self.price += gap * 0.05

        # GDP growth increases demand
        gdp_effect = (self.model.macro_gdp - 0.02) * 10.0
        self.price += gdp_effect

        # Floor
        self.price = max(10.0, self.price)


class BondAgent(mesa.Agent):
    """10-year Treasury bond yield tracker.

    Attributes:
        yield_rate: Current 10-year yield (decimal, e.g. 0.04 = 4%).
    """

    def __init__(self, model: mesa.Model) -> None:
        super().__init__(model)
        self.yield_rate: float = 0.04  # 4% baseline

    def step(self) -> None:
        """Advance one tick: track fed funds + risk premium."""
        # Risk premium rises with debt-to-GDP above 100%
        risk_premium = max(0.0, (self.model.macro_debt_to_gdp - 1.0) * 0.02)

        # Target = fed funds + risk premium + term premium
        target = self.model.macro_fed_rate + risk_premium + 0.015
        gap = target - self.yield_rate
        self.yield_rate += gap * 0.2

        # Floor at zero
        self.yield_rate = max(0.0, self.yield_rate)
