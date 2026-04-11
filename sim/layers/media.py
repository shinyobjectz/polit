"""MediaLayer -- amplifies / dampens political signals through news coverage.

Wraps a Mesa model of MediaAgents representing major media outlets.
Each tick the layer:
  1. Processes MediaCampaign events (advertising, disinformation).
  2. Amplifies existing approval deltas (negativity bias).
  3. Amplifies Scandal events through the media ecosystem.
"""

from __future__ import annotations

import mesa

from sim.agents.media_agent import MediaAgent
from sim.layers.base import SimulationLayer

# Default media ecosystem: ~8 outlets with varying reach/credibility/lean.
DEFAULT_OUTLETS: list[dict] = [
    {
        "name": "CNN",
        "media_type": "tv",
        "reach": 0.15,
        "credibility": 55,
        "editorial_lean": -30,
    },
    {
        "name": "Fox News",
        "media_type": "cable",
        "reach": 0.20,
        "credibility": 45,
        "editorial_lean": 60,
    },
    {
        "name": "MSNBC",
        "media_type": "cable",
        "reach": 0.10,
        "credibility": 50,
        "editorial_lean": -50,
    },
    {
        "name": "NYT",
        "media_type": "newspaper",
        "reach": 0.08,
        "credibility": 75,
        "editorial_lean": -20,
    },
    {
        "name": "WSJ",
        "media_type": "newspaper",
        "reach": 0.06,
        "credibility": 80,
        "editorial_lean": 10,
    },
    {
        "name": "AP/Reuters",
        "media_type": "wire",
        "reach": 0.12,
        "credibility": 90,
        "editorial_lean": 0,
    },
    {
        "name": "Social Media",
        "media_type": "social",
        "reach": 0.40,
        "credibility": 30,
        "editorial_lean": 0,
    },
    {
        "name": "Local News",
        "media_type": "tv",
        "reach": 0.25,
        "credibility": 65,
        "editorial_lean": 0,
    },
]


class MediaModel(mesa.Model):
    """Mesa model containing MediaAgent instances."""

    def __init__(self) -> None:
        super().__init__()
        for cfg in DEFAULT_OUTLETS:
            MediaAgent(
                self,
                name=cfg["name"],
                media_type=cfg["media_type"],
                reach=cfg["reach"],
                credibility=cfg["credibility"],
                editorial_lean=cfg["editorial_lean"],
            )

    def step(self) -> None:  # type: ignore[override]
        self.agents.shuffle_do("step")


def _normalize(raw: dict) -> dict:
    """Normalise a serde-tagged event to flat dict with 'type' key."""
    if "type" in raw:
        return raw
    if len(raw) == 1:
        variant = next(iter(raw))
        fields = raw[variant]
        if isinstance(fields, dict):
            return {"type": variant, **fields}
        return {"type": variant, "value": fields}
    return raw


class MediaLayer(SimulationLayer):
    """Simulation layer for media news propagation and belief cascade."""

    def __init__(self) -> None:
        self._model = MediaModel()

    def step(self, world_state: dict, events: list[dict], delta: dict) -> dict:
        # 1. Process MediaCampaign events --------------------------------
        for raw_event in events:
            event = _normalize(raw_event)
            if event.get("type") != "MediaCampaign":
                continue

            campaign_type = event.get("campaign_type", "")
            intensity = event.get("intensity", 0)
            source = event.get("source", "domestic")

            # Foreign sources have lower credibility multiplier
            cred_mult = 0.3 if source == "foreign" else 1.0

            for agent in self._model.agents:
                if campaign_type == "disinformation":
                    # Low-credibility outlets amplify disinformation more
                    amplification = (
                        (100 - agent.credibility) / 100 * intensity * cred_mult
                    )
                else:
                    amplification = (
                        agent.credibility / 100 * intensity * cred_mult
                    )

                # Belief shift weighted by reach
                belief_shift = amplification * agent.reach * 0.1

                if campaign_type == "disinformation":
                    delta["approval_president_delta"] -= belief_shift
                elif campaign_type == "advertising":
                    delta["approval_president_delta"] += belief_shift

        # 2. Amplify existing political signals --------------------------
        raw_approval = delta.get("approval_president_delta", 0)
        if abs(raw_approval) > 0.5:
            amplification = 0.0
            for agent in self._model.agents:
                # Negativity bias: bad news travels faster
                if raw_approval < 0:
                    amp = agent.reach * 0.3
                else:
                    amp = agent.reach * 0.1
                amplification += amp

            delta["approval_president_delta"] *= 1 + amplification

            if abs(delta["approval_president_delta"]) > 1.0:
                direction = (
                    "negative"
                    if delta["approval_president_delta"] < 0
                    else "positive"
                )
                delta.setdefault("narrative_seeds", []).append(
                    f"Media coverage amplifying {direction} political sentiment"
                )

        # 3. Scandal amplification ---------------------------------------
        for raw_event in events:
            event = _normalize(raw_event)
            if event.get("type") != "Scandal":
                continue

            severity = event.get("severity", 0)
            media_amp = event.get("media_amplification", 1.0)
            total_reach = sum(a.reach for a in self._model.agents)
            scandal_impact = severity * media_amp * total_reach * 0.5
            delta["approval_president_delta"] -= scandal_impact
            delta.setdefault("narrative_seeds", []).append(
                f"Scandal dominating news cycle (severity {severity:.1f})"
            )

        return delta
