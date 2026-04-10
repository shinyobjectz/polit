"""SectorLayer — wraps a Mesa SectorModel for sectoral economy simulation.

Nine sector agents (Energy, Tech, Pharma, Defense, Finance,
Manufacturing, Agriculture, Healthcare, Retail) respond to macro
conditions and event shocks, producing per-sector output / employment /
price deltas each tick.
"""

from __future__ import annotations

import mesa

from sim.agents.sector_agent import SectorAgent
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

# Sectors affected by tariff events.
TARIFF_SECTORS: set[str] = {"Manufacturing", "Tech", "Agriculture"}


class SectorModel(mesa.Model):
    """Mesa model containing one :class:`SectorAgent` per sector."""

    def __init__(self) -> None:
        super().__init__()
        for name in SECTOR_NAMES:
            SectorAgent(self, sector_name=name)

    def step(self) -> None:  # type: ignore[override]
        self.agents.shuffle_do("step")


class SectorLayer(SimulationLayer):
    """Simulation layer that drives the sectoral economy each tick.

    Reads macro conditions from *world_state*, applies event shocks,
    steps the Mesa model, and writes sector deltas + narrative seeds
    into *delta*.
    """

    def __init__(self) -> None:
        self._model = SectorModel()

    # ------------------------------------------------------------------
    # SimulationLayer interface
    # ------------------------------------------------------------------

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        macro = world_state.get("macro", {})

        # Pass macro conditions to agents
        for agent in self._model.agents:
            agent.macro_gdp_growth = (
                macro.get("gdp_growth", 0.02)
                + delta.get("gdp_growth_delta", 0)
            )
            agent.macro_fed_rate = macro.get("fed_funds_rate", 0.05)

        # Apply sector-specific shocks from events
        for event in events:
            etype = event.get("type", "")
            if etype == "SectorShock":
                sector = event.get("sector", "")
                severity = event.get("severity", 0)
                for agent in self._model.agents:
                    if agent.sector_name == sector:
                        agent.demand -= severity * 0.5
                        agent.supply -= severity * 0.3
            elif etype == "Tariff":
                rate = event.get("rate", 0)
                for agent in self._model.agents:
                    if agent.sector_name in TARIFF_SECTORS:
                        agent.price_level += rate * 0.5

        # Step the model
        self._model.step()

        # Collect sector deltas
        delta.setdefault("sector_deltas", {})
        for agent in self._model.agents:
            delta["sector_deltas"][agent.sector_name] = {
                "output_delta": agent.output - 1.0,
                "employment_delta": agent.employment - 1.0,
                "price_delta": agent.price_level - 1.0,
            }

        # Generate narrative seeds for significant changes
        delta.setdefault("narrative_seeds", [])
        for agent in self._model.agents:
            if abs(agent.output - 1.0) > 0.1:
                direction = "expanding" if agent.output > 1.0 else "contracting"
                delta["narrative_seeds"].append(
                    f"{agent.sector_name} sector {direction}"
                    f" ({(agent.output - 1) * 100:+.1f}%)"
                )

        return delta
