use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SectorDelta {
    pub output_delta: f64,
    pub employment_delta: f64,
    pub price_delta: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CountyDelta {
    pub income_delta: f64,
    pub unemployment_delta: f64,
    pub population_delta: f64,
    pub housing_price_delta: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IdeologyShift {
    pub demographic_group: String,
    pub direction: f64, // negative = left, positive = right
    pub magnitude: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ForeignPowerDelta {
    pub country: String,
    pub alignment_delta: f64,
    pub stability_delta: f64,
    pub trade_delta: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CorporateAction {
    pub corp_name: String,
    pub action_type: String, // lobby, donate, retaliate, invest
    pub target: String,
    pub intensity: f64,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldStateDelta {
    // Macro
    pub gdp_growth_delta: f64,
    pub inflation_delta: f64,
    pub unemployment_delta: f64,
    pub fed_funds_rate: f64,
    pub consumer_confidence_delta: f64,
    pub debt_to_gdp_delta: f64,
    // Sectors
    pub sector_deltas: HashMap<String, SectorDelta>,
    // Counties (only changed ones)
    pub county_deltas: HashMap<String, CountyDelta>,
    // Political
    pub approval_president_delta: f64,
    pub approval_congress_delta: f64,
    pub protest_risk_by_region: HashMap<String, f64>,
    pub voter_ideology_shifts: Vec<IdeologyShift>,
    // Markets
    pub sector_indices: HashMap<String, f64>,
    pub oil_price: f64,
    pub bond_yield_10yr: f64,
    // Geopolitical
    pub foreign_power_deltas: Vec<ForeignPowerDelta>,
    pub trade_balance_delta: f64,
    pub migration_pressure: HashMap<String, f64>,
    // Corporate
    pub corporate_actions: Vec<CorporateAction>,
    // DM hooks
    pub narrative_seeds: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_delta_roundtrips() {
        let delta = WorldStateDelta::default();
        let bytes = rmp_serde::to_vec(&delta).unwrap();
        let decoded: WorldStateDelta = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(delta, decoded);
    }

    #[test]
    fn delta_with_sector_changes_roundtrips() {
        let mut delta = WorldStateDelta::default();
        delta.gdp_growth_delta = -0.004;
        delta.inflation_delta = 0.012;
        delta.sector_deltas.insert(
            "energy".into(),
            SectorDelta {
                output_delta: 0.08,
                employment_delta: -0.02,
                price_delta: 0.15,
            },
        );
        delta
            .narrative_seeds
            .push("Gulf refineries showing capacity stress".into());
        let bytes = rmp_serde::to_vec(&delta).unwrap();
        let decoded: WorldStateDelta = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(delta, decoded);
    }
}
