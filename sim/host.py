"""Simulation host entry point. Rust calls tick() via PyO3."""

import msgpack

_layers: list = []


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


def _apply_exogenous_shocks(events: list[dict], delta: dict) -> None:
    """Apply direct event effects before layers process."""
    for event in events:
        etype = event.get("type", "")
        if etype == "SectorShock":
            sector = event.get("sector", "Unknown")
            severity = event.get("severity", 0.0)
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

    for layer in _layers:
        delta = layer.step(world_state, events, delta)

    return msgpack.packb(delta)
