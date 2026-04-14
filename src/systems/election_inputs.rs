use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IdeologySplit {
    pub left: f64,
    pub center: f64,
    pub right: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EconomicConditions {
    pub unemployment: f64,
    pub income_change: f64,
    pub anxiety: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ElectionInputs {
    pub ideology_distribution: HashMap<String, IdeologySplit>,
    pub turnout_propensity: HashMap<String, f64>,
    pub economic_conditions: HashMap<String, EconomicConditions>,
    pub approval_rating: HashMap<String, f64>,
    pub issue_salience: HashMap<String, f64>,
    pub swing_counties: Vec<String>,
    pub enthusiasm_gap: f64,
}
