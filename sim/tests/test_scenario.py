"""Tests for sim.scenario — TOML schema loader."""

import tempfile
import textwrap
from pathlib import Path

import pytest

from sim.scenario import (
    DEFAULT_SCENARIO,
    ScenarioConfig,
    load_scenario,
    scenario_to_world_state,
)


MINIMAL_TOML = Path(__file__).resolve().parents[2] / "game" / "scenarios" / "test_minimal.toml"


# ── Load minimal TOML, verify all fields parsed ─────────────────────


class TestLoadMinimal:
    def test_name(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.name == "Test Scenario"

    def test_description(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.description == "A minimal test scenario"

    def test_era(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.era == "modern"

    def test_year(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.year == 2024

    def test_macro_values(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.macro["gdp_growth"] == pytest.approx(0.025)
        assert cfg.macro["inflation"] == pytest.approx(0.031)
        assert cfg.macro["unemployment"] == pytest.approx(0.037)
        assert cfg.macro["fed_funds_rate"] == pytest.approx(0.0525)
        assert cfg.macro["consumer_confidence"] == pytest.approx(102.0)
        assert cfg.macro["debt_to_gdp"] == pytest.approx(1.23)

    def test_county_source(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.county_source == "fallback"

    def test_county_states_empty(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert cfg.county_states == []


# ── Missing sections use defaults ────────────────────────────────────


class TestDefaults:
    def test_empty_toml(self):
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(b"")
            f.flush()
            cfg = load_scenario(f.name)
        assert cfg.name == "Default"
        assert cfg.era == "modern"
        assert cfg.year == 2024
        assert cfg.macro["gdp_growth"] == pytest.approx(0.02)

    def test_only_scenario_section(self):
        toml_text = b'[scenario]\nname = "Custom"\n'
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(toml_text)
            f.flush()
            cfg = load_scenario(f.name)
        assert cfg.name == "Custom"
        # Macro should be defaults
        assert cfg.macro["unemployment"] == pytest.approx(0.045)

    def test_no_counties_section(self):
        toml_text = b'[scenario]\nname = "No Counties"\n'
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(toml_text)
            f.flush()
            cfg = load_scenario(f.name)
        assert cfg.county_source == "fallback"
        assert cfg.county_states == []


# ── Scheduled events grouped by week ─────────────────────────────────


class TestScheduledEvents:
    def test_events_grouped_by_week(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert 5 in cfg.scheduled_events
        assert 10 in cfg.scheduled_events

    def test_week5_event(self):
        cfg = load_scenario(MINIMAL_TOML)
        events = cfg.scheduled_events[5]
        assert len(events) == 1
        assert events[0]["type"] == "FiscalBill"
        assert events[0]["bill_type"] == "spending"
        assert events[0]["amount_gdp_pct"] == pytest.approx(0.02)

    def test_week10_event(self):
        cfg = load_scenario(MINIMAL_TOML)
        events = cfg.scheduled_events[10]
        assert len(events) == 1
        assert events[0]["type"] == "SectorShock"
        assert events[0]["sector"] == "Energy"
        assert events[0]["severity"] == pytest.approx(2.0)

    def test_week_not_in_event_dict(self):
        """The 'week' key should be removed from event dicts after grouping."""
        cfg = load_scenario(MINIMAL_TOML)
        for week_events in cfg.scheduled_events.values():
            for event in week_events:
                assert "week" not in event

    def test_multiple_events_same_week(self):
        toml_text = textwrap.dedent("""\
            [[events.scheduled]]
            week = 3
            type = "FiscalBill"
            bill_type = "tax"
            amount_gdp_pct = 0.01

            [[events.scheduled]]
            week = 3
            type = "SectorShock"
            sector = "Tech"
            severity = 1.5
        """).encode()
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(toml_text)
            f.flush()
            cfg = load_scenario(f.name)
        assert len(cfg.scheduled_events[3]) == 2


# ── Geopolitical overrides parsed correctly ──────────────────────────


class TestGeopoliticalOverrides:
    def test_china_override(self):
        cfg = load_scenario(MINIMAL_TOML)
        assert "China" in cfg.geopolitical_overrides
        assert cfg.geopolitical_overrides["China"]["alignment"] == pytest.approx(-0.5)

    def test_russia_override(self):
        cfg = load_scenario(MINIMAL_TOML)
        russia = cfg.geopolitical_overrides["Russia"]
        assert russia["alignment"] == pytest.approx(-0.7)
        assert russia["stability"] == 40

    def test_no_overrides(self):
        toml_text = b'[scenario]\nname = "Plain"\n'
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(toml_text)
            f.flush()
            cfg = load_scenario(f.name)
        assert cfg.geopolitical_overrides == {}


# ── scenario_to_world_state produces valid world_state dict ──────────


class TestWorldState:
    def test_world_state_has_week(self):
        cfg = load_scenario(MINIMAL_TOML)
        ws = scenario_to_world_state(cfg)
        assert ws["week"] == 1

    def test_world_state_has_macro(self):
        cfg = load_scenario(MINIMAL_TOML)
        ws = scenario_to_world_state(cfg)
        assert "macro" in ws
        assert ws["macro"]["gdp_growth"] == pytest.approx(0.025)

    def test_world_state_has_counties(self):
        cfg = load_scenario(MINIMAL_TOML)
        ws = scenario_to_world_state(cfg)
        assert "counties" in ws
        assert isinstance(ws["counties"], dict)

    def test_world_state_macro_is_copy(self):
        """Mutating world_state should not affect the config."""
        cfg = load_scenario(MINIMAL_TOML)
        ws = scenario_to_world_state(cfg)
        ws["macro"]["gdp_growth"] = 999.0
        assert cfg.macro["gdp_growth"] == pytest.approx(0.025)

    def test_default_scenario_world_state(self):
        ws = scenario_to_world_state(DEFAULT_SCENARIO)
        assert ws["week"] == 1
        assert ws["macro"]["gdp_growth"] == pytest.approx(0.02)


# ── Invalid TOML raises clear error ─────────────────────────────────


class TestErrors:
    def test_invalid_toml_syntax(self):
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(b"[invalid\nthis is not valid toml")
            f.flush()
            with pytest.raises(Exception):
                load_scenario(f.name)

    def test_file_not_found(self):
        with pytest.raises(FileNotFoundError):
            load_scenario("/nonexistent/path/scenario.toml")

    def test_invalid_era(self):
        toml_text = b'[scenario]\nera = "steampunk"\n'
        with tempfile.NamedTemporaryFile(suffix=".toml", mode="wb", delete=False) as f:
            f.write(toml_text)
            f.flush()
            with pytest.raises(ValueError, match="Invalid era"):
                load_scenario(f.name)


# ── DEFAULT_SCENARIO sentinel ───────────────────────────────────────


class TestDefaultScenario:
    def test_is_scenario_config(self):
        assert isinstance(DEFAULT_SCENARIO, ScenarioConfig)

    def test_equilibrium_macro(self):
        assert DEFAULT_SCENARIO.macro["gdp_growth"] == pytest.approx(0.02)
        assert DEFAULT_SCENARIO.macro["inflation"] == pytest.approx(0.02)
        assert DEFAULT_SCENARIO.macro["unemployment"] == pytest.approx(0.045)
