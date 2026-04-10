from abc import ABC, abstractmethod


class SimulationLayer(ABC):
    """Base class for simulation layers."""

    @abstractmethod
    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        """Process one tick. Mutate and return delta dict."""
        ...
