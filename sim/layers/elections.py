"""ElectionLayer -- on-demand election input computation.

This is NOT a per-tick layer. It provides simulation-grounded inputs for
the GDD section 19 vote calculation formula. The Rust side calls
``compute_election_inputs`` when an election is happening and feeds the
result into its own vote formula.
"""

from __future__ import annotations


class ElectionInputs:
    """Simulation-grounded inputs for the vote calculation formula."""

    def __init__(self) -> None:
        self.ideology_distribution: dict[str, dict[str, float]] = {}
        self.turnout_propensity: dict[str, float] = {}
        self.economic_conditions: dict[str, dict[str, float]] = {}
        self.approval_rating: dict[str, float] = {}
        self.issue_salience: dict[str, float] = {}
        self.swing_counties: list[str] = []
        self.enthusiasm_gap: float = 0.0


def compute_election_inputs(
    world_state: dict,
    delta: dict,
    voter_agents: list | None = None,
) -> ElectionInputs:
    """Compute election inputs from current simulation state.

    This does NOT compute votes -- the GDD's vote formula in Rust does that.
    This provides the inputs that make the formula's results grounded in
    simulation reality.

    Args:
        world_state: Current world state dict with ``macro`` and ``counties``.
        delta: Accumulated deltas from this tick's layer pipeline.
        voter_agents: Optional list of VoterAgent instances. When provided,
            ideology and turnout are computed from agent state; otherwise
            generic fallback estimates are used.

    Returns:
        Populated ``ElectionInputs`` instance.
    """
    inputs = ElectionInputs()

    macro = world_state.get("macro", {})
    counties = world_state.get("counties", {})

    # -- Economic conditions per county ------------------------------------

    for fips, county in counties.items():
        unemp = county.get(
            "unemployment_rate", macro.get("unemployment", 0.045)
        )
        income = county.get("median_household_income", 60000)  # noqa: F841

        inputs.economic_conditions[fips] = {
            "unemployment": unemp,
            "income_change": (
                delta.get("county_deltas", {})
                .get(fips, {})
                .get("income_delta", 0)
            ),
            "anxiety": max(0.0, min(1.0, (unemp - 0.04) * 5)),
        }

        # Incumbents punished in recession
        gdp = macro.get("gdp_growth", 0.02) + delta.get(
            "gdp_growth_delta", 0
        )
        incumbent_penalty = max(0.0, (0.02 - gdp) * 50)
        inputs.approval_rating[fips] = max(
            0.0,
            50 - incumbent_penalty
            - inputs.economic_conditions[fips]["anxiety"] * 20,
        )

    # -- Ideology & turnout ------------------------------------------------

    if voter_agents:
        # Group agents by county
        by_county: dict[str, list] = {}
        for agent in voter_agents:
            fips = agent.county_fips
            if fips not in by_county:
                by_county[fips] = []
            by_county[fips].append(agent)

        for fips, agents in by_county.items():
            total_share = sum(a.population_share for a in agents)
            if total_share == 0:
                continue

            # Weighted ideology distribution
            avg_ideology = (
                sum(a.ideology_score * a.population_share for a in agents)
                / total_share
            )
            inputs.ideology_distribution[fips] = {
                "left": (
                    sum(
                        a.population_share
                        for a in agents
                        if a.ideology_score < -0.2
                    )
                    / total_share
                ),
                "center": (
                    sum(
                        a.population_share
                        for a in agents
                        if -0.2 <= a.ideology_score <= 0.2
                    )
                    / total_share
                ),
                "right": (
                    sum(
                        a.population_share
                        for a in agents
                        if a.ideology_score > 0.2
                    )
                    / total_share
                ),
            }

            # Weighted turnout propensity
            inputs.turnout_propensity[fips] = (
                sum(a.turnout_propensity * a.population_share for a in agents)
                / total_share
            )

            # Swing county identification
            if abs(avg_ideology) < 0.15:
                inputs.swing_counties.append(fips)
    else:
        # Fallback: generic estimates when no voter agents available
        for fips in counties:
            inputs.turnout_propensity[fips] = 0.60
            inputs.ideology_distribution[fips] = {
                "left": 0.35,
                "center": 0.30,
                "right": 0.35,
            }

    # -- Enthusiasm gap ----------------------------------------------------

    avg_anxiety = (
        sum(ec.get("anxiety", 0) for ec in inputs.economic_conditions.values())
        / max(1, len(inputs.economic_conditions))
    )
    # Anxiety hurts incumbent enthusiasm -> negative gap
    inputs.enthusiasm_gap = -avg_anxiety * 0.5

    # -- Issue salience ----------------------------------------------------

    gdp = macro.get("gdp_growth", 0.02) + delta.get("gdp_growth_delta", 0)
    unemp = macro.get("unemployment", 0.045) + delta.get(
        "unemployment_delta", 0
    )
    inflation = macro.get("inflation", 0.02) + delta.get(
        "inflation_delta", 0
    )

    inputs.issue_salience = {
        "economy": min(
            1.0, abs(gdp - 0.02) * 20 + abs(unemp - 0.045) * 10
        ),
        "inflation": min(1.0, max(0.0, (inflation - 0.03) * 30)),
        "jobs": min(1.0, max(0.0, (unemp - 0.05) * 15)),
        "healthcare": 0.3,
        "immigration": 0.2,
        "foreign_policy": 0.1,
    }

    return inputs


def election_inputs_to_dict(inputs: ElectionInputs) -> dict:
    """Serialize ElectionInputs for MessagePack transport to Rust."""
    return {
        "ideology_distribution": inputs.ideology_distribution,
        "turnout_propensity": inputs.turnout_propensity,
        "economic_conditions": inputs.economic_conditions,
        "approval_rating": inputs.approval_rating,
        "issue_salience": inputs.issue_salience,
        "swing_counties": inputs.swing_counties,
        "enthusiasm_gap": inputs.enthusiasm_gap,
    }
