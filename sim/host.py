"""Simulation host entry point. Rust calls tick() via PyO3."""

import msgpack

from sim.layers.household import HouseholdLayer
from sim.layers.macro_economy import MacroEconomyLayer
from sim.layers.sectors import SectorLayer

_layers: list = [
    MacroEconomyLayer(),  # 1. macro runs first — GDP, inflation, unemployment
    SectorLayer(),        # 2. sectors respond to macro conditions + shocks
    HouseholdLayer(),     # 3. household reads macro + sector output
]


def reset_layers() -> None:
    """Reset all layers to fresh state. Used by tests."""
    global _layers
    _layers = [
        MacroEconomyLayer(),
        SectorLayer(),
        HouseholdLayer(),
    ]


def _default_delta() -> dict:
    return {
        "gdp_growth_delta": 0.0,
        "inflation_delta": 0.0,
        "unemployment_delta": 0.0,
        "fed_funds_rate": 0.0,
        "consumer_confidence_delta": 0.0,
        "debt_to_gdp_delta": 0.0,
        "sector_deltas": {},
        "county_deltas": {},
        "approval_president_delta": 0.0,
        "approval_congress_delta": 0.0,
        "protest_risk_by_region": {},
        "voter_ideology_shifts": [],
        "sector_indices": {},
        "oil_price": 0.0,
        "bond_yield_10yr": 0.0,
        "foreign_power_deltas": [],
        "trade_balance_delta": 0.0,
        "migration_pressure": {},
        "corporate_actions": [],
        "narrative_seeds": [],
    }


def _normalize_event(raw: dict) -> dict:
    """Normalise a Rust serde-serialised enum into a flat dict with a 'type' key.

    rmp_serde serialises externally-tagged enums as ``{"VariantName": {fields...}}``.
    This helper converts that into ``{"type": "VariantName", **fields}`` so the
    rest of the pipeline can use a simple ``event["type"]`` lookup.

    If the event already has a ``"type"`` key (e.g. hand-crafted test data) it is
    returned unchanged.
    """
    if "type" in raw:
        return raw
    # Externally tagged: single-key dict whose key is the variant name
    if len(raw) == 1:
        variant_name = next(iter(raw))
        fields = raw[variant_name]
        if isinstance(fields, dict):
            return {"type": variant_name, **fields}
        # Unit variant or newtype — wrap the value
        return {"type": variant_name, "value": fields}
    return raw


def _apply_exogenous_shocks(events: list[dict], delta: dict) -> None:
    """Apply direct event effects before layers process."""
    for raw_event in events:
        event = _normalize_event(raw_event)
        etype = event.get("type", "")
        if etype == "SectorShock":
            sector = event.get("sector", "Unknown")
            severity = event.get("severity", 0.0)
            # Sector may arrive as a serde-tagged enum too (e.g. {"Energy": None}
            # or just a string). Normalise to string.
            if isinstance(sector, dict) and len(sector) == 1:
                sector = next(iter(sector))
            delta["sector_deltas"][sector] = {
                "output_delta": -severity * 0.1,
                "employment_delta": -severity * 0.05,
                "price_delta": severity * 0.08,
            }
            delta["narrative_seeds"].append(
                f"{sector} sector experiencing shock (severity {severity:.1f})"
            )


def tick(world_state_bytes: bytes, events_bytes: bytes) -> bytes:
    world_state = msgpack.unpackb(world_state_bytes, raw=False)
    events = msgpack.unpackb(events_bytes, raw=False)

    delta = _default_delta()
    _apply_exogenous_shocks(events, delta)

    # Layers run in dependency order: macro → sectors → household.
    # Each layer reads the accumulated delta from previous layers so
    # cross-layer effects cascade within a single tick:
    #   macro produces gdp_growth_delta, fed_funds_rate, unemployment_delta
    #   → sectors read those to adjust output/employment
    #   → household reads sector_deltas + macro deltas for income effects
    for layer in _layers:
        delta = layer.step(world_state, events, delta)

    return msgpack.packb(delta)
