//! County and household population data models (Rust-side mirror).
//!
//! These structs mirror the Python `sim.models.population` dataclasses
//! so that county data can cross the Python→Rust bridge via msgpack.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statistical household profile for a county (not individual households).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseholdProfile {
    /// Five income quintile shares, summing to 1.0.
    pub income_quintile_distribution: Vec<f64>,
    /// Education level → population share (no_hs, hs, some_college, bachelors, graduate).
    pub education_distribution: HashMap<String, f64>,
    /// Age bracket → population share.
    pub age_distribution: HashMap<String, f64>,
    /// Race/ethnicity → population share.
    pub race_distribution: HashMap<String, f64>,
    /// Home ownership rate (0.0–1.0).
    pub housing_own_rent_split: f64,
    /// Food insecurity rate (0.0–1.0).
    pub food_insecurity_rate: f64,
    /// Health insurance coverage rate (0.0–1.0).
    pub insurance_coverage_rate: f64,
}

impl Default for HouseholdProfile {
    fn default() -> Self {
        Self {
            income_quintile_distribution: vec![0.2; 5],
            education_distribution: HashMap::new(),
            age_distribution: HashMap::new(),
            race_distribution: HashMap::new(),
            housing_own_rent_split: 0.65,
            food_insecurity_rate: 0.10,
            insurance_coverage_rate: 0.90,
        }
    }
}

/// County-level population and economic data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountyData {
    pub fips: String,
    pub state: String,
    pub name: String,
    pub population: u64,
    pub area_sq_miles: f64,
    pub median_household_income: f64,
    pub unemployment_rate: f64,
    /// Sector → employment share.
    pub major_industries: HashMap<String, f64>,
    pub housing_vacancy_rate: f64,
    pub unionization_rate: f64,
    /// Political lean index: -1.0 (left) to +1.0 (right).
    pub political_lean_index: f64,
    /// One of: urban, suburban, exurban, rural.
    pub urban_rural: String,
    /// Party → registration share (dem, rep, ind).
    pub voter_registration: HashMap<String, f64>,
    pub turnout_propensity: f64,
    pub households: HouseholdProfile,
}

impl Default for CountyData {
    fn default() -> Self {
        Self {
            fips: String::new(),
            state: String::new(),
            name: String::new(),
            population: 0,
            area_sq_miles: 0.0,
            median_household_income: 60_000.0,
            unemployment_rate: 0.04,
            major_industries: HashMap::new(),
            housing_vacancy_rate: 0.07,
            unionization_rate: 0.10,
            political_lean_index: 0.0,
            urban_rural: "suburban".to_string(),
            voter_registration: HashMap::new(),
            turnout_propensity: 0.60,
            households: HouseholdProfile::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_household_quintiles_sum_to_one() {
        let hp = HouseholdProfile::default();
        assert_eq!(hp.income_quintile_distribution.len(), 5);
        let sum: f64 = hp.income_quintile_distribution.iter().sum();
        assert!((sum - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn county_data_msgpack_roundtrip() {
        let mut industries = HashMap::new();
        industries.insert("healthcare".to_string(), 0.18);
        industries.insert("education".to_string(), 0.12);

        let mut voter_reg = HashMap::new();
        voter_reg.insert("dem".to_string(), 0.42);
        voter_reg.insert("rep".to_string(), 0.35);
        voter_reg.insert("ind".to_string(), 0.23);

        let county = CountyData {
            fips: "39049".to_string(),
            state: "OH".to_string(),
            name: "Franklin County".to_string(),
            population: 1_323_807,
            area_sq_miles: 543.5,
            median_household_income: 62_000.0,
            unemployment_rate: 0.04,
            major_industries: industries,
            housing_vacancy_rate: 0.06,
            unionization_rate: 0.09,
            political_lean_index: -0.05,
            urban_rural: "urban".to_string(),
            voter_registration: voter_reg,
            turnout_propensity: 0.63,
            households: HouseholdProfile::default(),
        };

        let packed = rmp_serde::to_vec(&county).expect("serialize");
        let restored: CountyData = rmp_serde::from_slice(&packed).expect("deserialize");
        assert_eq!(county, restored);
    }
}
