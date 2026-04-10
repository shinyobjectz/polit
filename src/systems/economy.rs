use serde::{Deserialize, Serialize};

/// Layered economic simulation
/// Layer 1: Surface indicators (what players see)
/// Layer 2: Macro model (what drives the simulation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    // Layer 1: Surface
    pub overall_health: f64,      // -1.0 (crisis) to 1.0 (boom)
    pub approval_on_economy: f64, // 0-100

    // Layer 2: Macro
    pub gdp_growth: f64,   // annual % change
    pub unemployment: f64, // percentage
    pub inflation: f64,    // annual %
    pub federal_funds_rate: f64,
    pub national_debt_gdp: f64,   // debt-to-GDP ratio
    pub consumer_confidence: f64, // 0-100
    pub trade_balance: f64,       // billions, negative = deficit

    // Policy effects queue (lagged)
    pub pending_effects: Vec<PolicyEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEffect {
    pub source: String,       // which law/action caused this
    pub variable: String,     // which economic variable
    pub delta: f64,           // change amount
    pub weeks_remaining: u32, // countdown until applied
    pub description: String,
}

impl Default for EconomyState {
    fn default() -> Self {
        Self {
            overall_health: 0.3,
            approval_on_economy: 55.0,
            gdp_growth: 2.1,
            unemployment: 4.8,
            inflation: 3.2,
            federal_funds_rate: 5.25,
            national_debt_gdp: 1.24,
            consumer_confidence: 62.0,
            trade_balance: -68.0,
            pending_effects: vec![],
        }
    }
}

impl EconomyState {
    /// Tick the economy forward one week
    pub fn tick(&mut self) {
        // Collect effects to apply (separate from mutable iteration)
        let mut to_apply: Vec<(String, f64)> = Vec::new();
        let mut to_remove: Vec<usize> = Vec::new();

        for (i, effect) in self.pending_effects.iter_mut().enumerate() {
            if effect.weeks_remaining == 0 {
                to_apply.push((effect.variable.clone(), effect.delta));
                to_remove.push(i);
            } else {
                effect.weeks_remaining -= 1;
            }
        }

        // Apply collected effects
        for (var, delta) in to_apply {
            self.apply_effect(&var, delta);
        }
        // Remove applied (reverse order)
        for i in to_remove.into_iter().rev() {
            self.pending_effects.remove(i);
        }

        // Natural economic drift
        self.natural_drift();

        // Update surface indicators
        self.update_surface();
    }

    fn apply_effect(&mut self, variable: &str, delta: f64) {
        match variable {
            "gdp_growth" => self.gdp_growth += delta,
            "unemployment" => self.unemployment = (self.unemployment + delta).max(0.0),
            "inflation" => self.inflation += delta,
            "consumer_confidence" => {
                self.consumer_confidence = (self.consumer_confidence + delta).clamp(0.0, 100.0)
            }
            "federal_funds_rate" => {
                self.federal_funds_rate = (self.federal_funds_rate + delta).max(0.0)
            }
            "trade_balance" => self.trade_balance += delta,
            "national_debt_gdp" => self.national_debt_gdp += delta,
            _ => {}
        }
    }

    fn natural_drift(&mut self) {
        // Small random-ish drift (deterministic based on current state)
        // Mean-reverting towards equilibrium
        self.gdp_growth += (2.0 - self.gdp_growth) * 0.02;
        self.unemployment += (5.0 - self.unemployment) * 0.01;
        self.inflation += (2.5 - self.inflation) * 0.01;
        self.consumer_confidence += (60.0 - self.consumer_confidence) * 0.02;
    }

    fn update_surface(&mut self) {
        // Compute overall health from macro indicators
        let gdp_score = (self.gdp_growth - 1.0) / 4.0; // 1% = -0, 5% = 1.0
        let unemp_score = (6.0 - self.unemployment) / 6.0; // 0% = 1.0, 6% = 0
        let inf_score = (4.0 - self.inflation) / 4.0; // 0% = 1.0, 4% = 0
        let conf_score = self.consumer_confidence / 100.0;

        self.overall_health =
            (gdp_score * 0.3 + unemp_score * 0.3 + inf_score * 0.2 + conf_score * 0.2)
                .clamp(-1.0, 1.0);

        // Approval tracks health with lag
        let target_approval = 50.0 + self.overall_health * 30.0;
        self.approval_on_economy += (target_approval - self.approval_on_economy) * 0.1;
        self.approval_on_economy = self.approval_on_economy.clamp(0.0, 100.0);
    }

    /// Apply a simulation delta from the Python bridge
    pub fn apply_delta(&mut self, delta: &crate::systems::world_delta::WorldStateDelta) {
        self.gdp_growth += delta.gdp_growth_delta;
        self.unemployment += delta.unemployment_delta;
        self.inflation += delta.inflation_delta;
        self.consumer_confidence += delta.consumer_confidence_delta;
        // Clamp to reasonable ranges
        self.unemployment = self.unemployment.clamp(0.0, 0.5);
        self.inflation = self.inflation.clamp(-0.1, 1.0);
        self.consumer_confidence = self.consumer_confidence.clamp(0.0, 200.0);
        // Update surface indicators after applying delta
        self.update_surface();
    }

    /// Queue a policy effect with lag
    pub fn queue_effect(
        &mut self,
        source: &str,
        variable: &str,
        delta: f64,
        lag_weeks: u32,
        description: &str,
    ) {
        self.pending_effects.push(PolicyEffect {
            source: source.to_string(),
            variable: variable.to_string(),
            delta,
            weeks_remaining: lag_weeks,
            description: description.to_string(),
        });
    }

    /// Get a summary string for AI context
    pub fn summary(&self) -> String {
        let health_word = if self.overall_health > 0.3 {
            "strong"
        } else if self.overall_health > -0.1 {
            "stable"
        } else if self.overall_health > -0.5 {
            "struggling"
        } else {
            "in crisis"
        };

        format!(
            "Economy is {}. GDP growth: {:.1}%, Unemployment: {:.1}%, Inflation: {:.1}%, Consumer confidence: {:.0}/100",
            health_word, self.gdp_growth, self.unemployment, self.inflation, self.consumer_confidence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let econ = EconomyState::default();
        assert!(econ.gdp_growth > 0.0);
        assert!(econ.unemployment > 0.0 && econ.unemployment < 20.0);
    }

    #[test]
    fn test_tick_drift() {
        let mut econ = EconomyState::default();
        let initial_gdp = econ.gdp_growth;
        for _ in 0..10 {
            econ.tick();
        }
        // Should drift slightly toward equilibrium
        assert!((econ.gdp_growth - initial_gdp).abs() < 1.0);
    }

    #[test]
    fn test_policy_effect_lag() {
        let mut econ = EconomyState::default();
        let initial_unemp = econ.unemployment;
        econ.queue_effect(
            "min_wage_increase",
            "unemployment",
            0.3,
            3,
            "Min wage effect",
        );

        // Tick 3 weeks — lag counts down: 3→2→1→0
        econ.tick(); // 3→2
        econ.tick(); // 2→1
        econ.tick(); // 1→0
        assert_eq!(econ.pending_effects.len(), 1); // applied on NEXT tick when remaining==0

        econ.tick(); // remaining==0, applied and removed
        assert_eq!(econ.pending_effects.len(), 0);
        // Unemployment should have increased (plus some drift)
        assert!(econ.unemployment > initial_unemp);
    }

    #[test]
    fn test_surface_indicators() {
        let mut econ = EconomyState {
            gdp_growth: 4.0,
            unemployment: 3.0,
            inflation: 2.0,
            consumer_confidence: 80.0,
            ..Default::default()
        };
        econ.update_surface();
        assert!(econ.overall_health > 0.5); // Good economy
        assert!(econ.approval_on_economy > 50.0);
    }

    #[test]
    fn test_crisis_economy() {
        let mut econ = EconomyState {
            gdp_growth: -2.0,
            unemployment: 10.0,
            inflation: 8.0,
            consumer_confidence: 20.0,
            ..Default::default()
        };
        econ.update_surface();
        assert!(econ.overall_health < 0.0); // Bad economy
    }

    #[test]
    fn test_summary() {
        let econ = EconomyState::default();
        let summary = econ.summary();
        assert!(summary.contains("GDP growth"));
        assert!(summary.contains("Unemployment"));
    }

    #[test]
    fn test_multiple_effects() {
        let mut econ = EconomyState::default();
        econ.queue_effect("tax_cut", "gdp_growth", 0.5, 2, "Tax cut boost");
        econ.queue_effect("spending_bill", "national_debt_gdp", 0.05, 1, "Spending");
        assert_eq!(econ.pending_effects.len(), 2);

        econ.tick(); // spending: 1→0, tax: 2→1
        assert_eq!(econ.pending_effects.len(), 2); // both still pending

        econ.tick(); // spending: applied+removed, tax: 1→0
        assert_eq!(econ.pending_effects.len(), 1);

        econ.tick(); // tax: applied+removed
        assert_eq!(econ.pending_effects.len(), 0);
    }
}
