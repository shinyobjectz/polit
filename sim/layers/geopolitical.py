"""GeopoliticalLayer — wraps a Mesa model of foreign powers.

Processes Conflict, Sanction, AllianceShift, and Tariff events,
steps country agents, and emits foreign-power deltas, trade-balance
changes, and migration-pressure estimates each tick.
"""

from __future__ import annotations

import mesa

from sim.agents.country_agent import CountryAgent, compute_migration_pressure
from sim.layers.base import SimulationLayer

# ------------------------------------------------------------------
# Default country data
# ------------------------------------------------------------------

TIER_1_COUNTRIES: list[dict] = [
    {
        "name": "China",
        "tier": 1,
        "alignment": -0.4,
        "military": 80,
        "economic": 95,
        "diplomatic": 70,
        "nuclear": 70,
        "stability": 75,
        "trade_with_us": {"imports": 500, "exports": 150},
    },
    {
        "name": "Russia",
        "tier": 1,
        "alignment": -0.7,
        "military": 75,
        "economic": 40,
        "diplomatic": 50,
        "nuclear": 95,
        "stability": 55,
        "trade_with_us": {"imports": 20, "exports": 5},
    },
    {
        "name": "UK",
        "tier": 1,
        "alignment": 0.8,
        "military": 60,
        "economic": 65,
        "diplomatic": 85,
        "nuclear": 40,
        "stability": 85,
        "trade_with_us": {"imports": 60, "exports": 70},
    },
    {
        "name": "EU",
        "tier": 1,
        "alignment": 0.6,
        "military": 50,
        "economic": 90,
        "diplomatic": 80,
        "nuclear": 30,
        "stability": 70,
        "trade_with_us": {"imports": 400, "exports": 300},
    },
    {
        "name": "Israel",
        "tier": 1,
        "alignment": 0.7,
        "military": 55,
        "economic": 35,
        "diplomatic": 40,
        "nuclear": 30,
        "stability": 65,
        "trade_with_us": {"imports": 20, "exports": 25},
    },
    {
        "name": "Iran",
        "tier": 1,
        "alignment": -0.8,
        "military": 45,
        "economic": 20,
        "diplomatic": 25,
        "nuclear": 15,
        "stability": 45,
        "trade_with_us": {"imports": 0, "exports": 0},
    },
]

TIER_2_COUNTRIES: list[dict] = [
    {
        "name": "Japan",
        "tier": 2,
        "alignment": 0.7,
        "military": 40,
        "economic": 75,
        "diplomatic": 65,
        "nuclear": 0,
        "stability": 85,
        "trade_with_us": {"imports": 140, "exports": 75},
    },
    {
        "name": "India",
        "tier": 2,
        "alignment": 0.3,
        "military": 55,
        "economic": 50,
        "diplomatic": 45,
        "nuclear": 40,
        "stability": 60,
        "trade_with_us": {"imports": 80, "exports": 40},
    },
    {
        "name": "Saudi Arabia",
        "tier": 2,
        "alignment": 0.4,
        "military": 35,
        "economic": 55,
        "diplomatic": 35,
        "nuclear": 0,
        "stability": 65,
        "trade_with_us": {"imports": 15, "exports": 20},
    },
    {
        "name": "Mexico",
        "tier": 2,
        "alignment": 0.5,
        "military": 20,
        "economic": 40,
        "diplomatic": 40,
        "nuclear": 0,
        "stability": 55,
        "trade_with_us": {"imports": 400, "exports": 300},
    },
    {
        "name": "Brazil",
        "tier": 2,
        "alignment": 0.3,
        "military": 25,
        "economic": 45,
        "diplomatic": 35,
        "nuclear": 0,
        "stability": 55,
        "trade_with_us": {"imports": 35, "exports": 40},
    },
    {
        "name": "South Korea",
        "tier": 2,
        "alignment": 0.7,
        "military": 45,
        "economic": 60,
        "diplomatic": 50,
        "nuclear": 0,
        "stability": 80,
        "trade_with_us": {"imports": 110, "exports": 65},
    },
]

ALL_DEFAULT_COUNTRIES: list[dict] = TIER_1_COUNTRIES + TIER_2_COUNTRIES


# ------------------------------------------------------------------
# Mesa model
# ------------------------------------------------------------------


class GeopoliticalModel(mesa.Model):
    """Mesa model containing one :class:`CountryAgent` per foreign power."""

    def __init__(self, countries: list[dict] | None = None) -> None:
        super().__init__()
        for spec in (countries or ALL_DEFAULT_COUNTRIES):
            CountryAgent(
                self,
                name=spec["name"],
                tier=spec["tier"],
                alignment=spec["alignment"],
                military=spec["military"],
                economic=spec["economic"],
                diplomatic=spec["diplomatic"],
                nuclear=spec["nuclear"],
                stability=spec["stability"],
                trade_with_us=spec["trade_with_us"],
            )

    def step(self) -> None:  # type: ignore[override]
        self.agents.shuffle_do("step")

    def get_country(self, name: str) -> CountryAgent | None:
        """Look up a country agent by name (case-insensitive)."""
        lower = name.lower()
        for agent in self.agents:
            if agent.name.lower() == lower:
                return agent
        return None


# ------------------------------------------------------------------
# Simulation layer
# ------------------------------------------------------------------


class GeopoliticalLayer(SimulationLayer):
    """Processes geopolitical events and steps foreign-power agents.

    Event types handled:

    * **Conflict** — ``{"type": "Conflict", "countries": [...], "severity": float}``
    * **Sanction** — ``{"type": "Sanction", "target": str, "severity": float}``
    * **AllianceShift** — ``{"type": "AllianceShift", "country": str, "shift": float}``
    * **Tariff** — ``{"type": "Tariff", "target": str, "rate": float}``
    """

    def __init__(self, countries: list[dict] | None = None) -> None:
        self._model = GeopoliticalModel(countries)

    # ------------------------------------------------------------------
    # Event handlers
    # ------------------------------------------------------------------

    def _handle_conflict(self, event: dict) -> list[str]:
        seeds: list[str] = []
        severity = event.get("severity", 0.5)
        for name in event.get("countries", []):
            agent = self._model.get_country(name)
            if agent is None:
                continue
            agent.stability -= severity * 10
            agent.stability = max(0, agent.stability)
            agent.alignment -= severity * 0.1
            agent.alignment = max(-1.0, agent.alignment)
            seeds.append(
                f"Conflict involving {name} (stability "
                f"{agent.stability:.0f}, alignment {agent.alignment:+.2f})"
            )
        return seeds

    def _handle_sanction(self, event: dict) -> list[str]:
        seeds: list[str] = []
        target = event.get("target", "")
        severity = event.get("severity", 0.5)
        agent = self._model.get_country(target)
        if agent is None:
            return seeds
        # Reduce trade volumes
        reduction = severity * 0.3  # up to 30% per event
        agent.trade_with_us["imports"] *= (1 - reduction)
        agent.trade_with_us["exports"] *= (1 - reduction)
        # Shift alignment negative
        agent.alignment -= severity * 0.15
        agent.alignment = max(-1.0, agent.alignment)
        seeds.append(
            f"Sanctions on {target} — trade reduced {reduction * 100:.0f}%, "
            f"alignment {agent.alignment:+.2f}"
        )
        return seeds

    def _handle_alliance_shift(self, event: dict) -> list[str]:
        seeds: list[str] = []
        country = event.get("country", "")
        shift = event.get("shift", 0.0)
        agent = self._model.get_country(country)
        if agent is None:
            return seeds
        agent.alignment = max(-1.0, min(1.0, agent.alignment + shift))
        seeds.append(
            f"Alliance shift with {country}: alignment {agent.alignment:+.2f}"
        )
        return seeds

    def _handle_tariff(self, event: dict) -> list[str]:
        seeds: list[str] = []
        target = event.get("target", "")
        rate = event.get("rate", 0.0)
        agent = self._model.get_country(target)
        if agent is None:
            return seeds
        # Reduce bilateral trade proportional to tariff rate
        trade_reduction = min(rate * 0.5, 0.5)  # cap at 50%
        agent.trade_with_us["imports"] *= (1 - trade_reduction)
        agent.trade_with_us["exports"] *= (1 - trade_reduction)
        # Alignment drifts negative from trade friction
        agent.alignment -= rate * 0.05
        agent.alignment = max(-1.0, agent.alignment)
        seeds.append(
            f"Tariff on {target} at {rate * 100:.0f}% — trade reduced, "
            f"alignment {agent.alignment:+.2f}"
        )
        return seeds

    # ------------------------------------------------------------------
    # SimulationLayer interface
    # ------------------------------------------------------------------

    _EVENT_HANDLERS = {
        "Conflict": "_handle_conflict",
        "Sanction": "_handle_sanction",
        "AllianceShift": "_handle_alliance_shift",
        "Tariff": "_handle_tariff",
    }

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        delta.setdefault("narrative_seeds", [])

        # 1. Process events
        for event in events:
            handler_name = self._EVENT_HANDLERS.get(event.get("type", ""))
            if handler_name is not None:
                seeds = getattr(self, handler_name)(event)
                delta["narrative_seeds"].extend(seeds)

        # 2. Step all country agents (slow drift)
        self._model.step()

        # 3. Compute aggregated outputs
        foreign_power_deltas: dict[str, dict] = {}
        total_imports = 0.0
        total_exports = 0.0
        migration_pressure: dict[str, float] = {}

        for agent in self._model.agents:
            foreign_power_deltas[agent.name] = {
                "alignment": agent.alignment,
                "stability": agent.stability,
                "military": agent.military,
                "trade_imports": agent.trade_with_us["imports"],
                "trade_exports": agent.trade_with_us["exports"],
            }
            total_imports += agent.trade_with_us["imports"]
            total_exports += agent.trade_with_us["exports"]
            migration_pressure[agent.name] = compute_migration_pressure(agent)

        delta["foreign_power_deltas"] = foreign_power_deltas
        delta["trade_balance_delta"] = total_exports - total_imports
        delta["migration_pressure"] = migration_pressure

        return delta
