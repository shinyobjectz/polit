import msgpack
from sim.host import tick


def test_tick_returns_valid_delta():
    """Empty world state + no events should return a valid default delta."""
    world_state = msgpack.packb({
        "week": 1,
        "counties": {},
        "macro": {
            "gdp_growth": 0.02,
            "inflation": 0.03,
            "unemployment": 0.04,
            "fed_funds_rate": 0.05,
            "consumer_confidence": 100.0,
            "debt_to_gdp": 1.2,
        },
    })
    events = msgpack.packb([])

    result_bytes = tick(world_state, events)
    delta = msgpack.unpackb(result_bytes)

    assert isinstance(delta, dict)
    assert "gdp_growth_delta" in delta
    assert "narrative_seeds" in delta
    assert isinstance(delta["narrative_seeds"], list)


def test_tick_processes_sector_shock():
    """A sector shock event should produce nonzero sector deltas."""
    world_state = msgpack.packb({
        "week": 1,
        "counties": {},
        "macro": {
            "gdp_growth": 0.02,
            "inflation": 0.03,
            "unemployment": 0.04,
            "fed_funds_rate": 0.05,
            "consumer_confidence": 100.0,
            "debt_to_gdp": 1.2,
        },
    })
    events = msgpack.packb([{
        "type": "SectorShock",
        "sector": "Energy",
        "region": "Ohio",
        "severity": 0.7,
    }])

    result_bytes = tick(world_state, events)
    delta = msgpack.unpackb(result_bytes)

    assert "sector_deltas" in delta
    assert len(delta["sector_deltas"]) > 0
