"""Simulation host entry point. Rust calls tick() via PyO3."""


def tick(world_state_bytes: bytes, events_bytes: bytes) -> bytes:
    """Process one simulation tick.

    Args:
        world_state_bytes: MessagePack-encoded world state snapshot
        events_bytes: MessagePack-encoded list of SimEvents

    Returns:
        MessagePack-encoded WorldStateDelta
    """
    raise NotImplementedError("Simulation host not yet implemented")
