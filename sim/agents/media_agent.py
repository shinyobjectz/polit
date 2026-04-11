"""MediaAgent -- Mesa 3.x agent representing a media organization.

Each agent represents a media outlet (TV network, newspaper, social media
aggregate, etc.) that amplifies or dampens political signals based on its
editorial lean, credibility, and reach.
"""

from __future__ import annotations

import mesa


class MediaAgent(mesa.Agent):
    """A media organization in the Mesa model.

    Attributes:
        name: Human-readable outlet name (e.g. ``"CNN"``).
        media_type: Category — ``tv``, ``cable``, ``digital``, ``newspaper``,
            ``social``, or ``wire``.
        reach: Fraction of the population this outlet reaches (0-1).
        credibility: Perceived credibility score (0-100).
        editorial_lean: Position on the left-right spectrum
            (-100 far-left to +100 far-right).
        current_stories: Stories being amplified during the current tick.
    """

    def __init__(
        self,
        model: mesa.Model,
        name: str,
        media_type: str,
        reach: float,
        credibility: float,
        editorial_lean: float,
    ) -> None:
        super().__init__(model)
        self.name = name
        self.media_type = media_type
        self.reach = reach
        self.credibility = credibility
        self.editorial_lean = editorial_lean
        self.current_stories: list[dict] = []

    def step(self) -> None:
        """Advance the media agent by one tick.

        Stories are injected externally by the MediaLayer; the agent
        itself is a passive data holder for now.
        """
        # Clear stories from previous tick
        self.current_stories = []
