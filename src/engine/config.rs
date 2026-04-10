use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level game configuration loaded from TOML files
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub balance: BalanceConfig,
    pub difficulty: DifficultyMode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BalanceConfig {
    pub action_points: ActionPointsConfig,
    pub action_costs: ActionCostsConfig,
    pub dice: DiceConfig,
    pub cards: CardsConfig,
    pub coherence: CoherenceConfig,
    pub stress: StressConfig,
    pub elections: ElectionsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionPointsConfig {
    pub local: i32,
    pub state: i32,
    pub federal_house: i32,
    pub federal_senate: i32,
    pub governor: i32,
    pub president: i32,
    pub bureaucratic_low: i32,
    pub bureaucratic_high: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionCostsConfig {
    pub meet_in_person: i32,
    pub phone_call: i32,
    pub speech: i32,
    pub campaign: i32,
    pub draft_legislation: i32,
    pub research: i32,
    pub scheme: i32,
    pub press_conference: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiceConfig {
    pub sides: u32,
    pub crit_success: u32,
    pub crit_failure: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CardsConfig {
    pub max_deck_size: u32,
    pub evolution_threshold: u32,
    pub neglect_threshold: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoherenceConfig {
    pub principled_threshold: i32,
    pub flipflopper_threshold: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StressConfig {
    pub crisis_stress: i32,
    pub scandal_stress: i32,
    pub overwork_threshold: i32,
    pub burnout_threshold: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ElectionsConfig {
    pub campaign_weeks_primary: u32,
    pub campaign_weeks_general: u32,
    pub incumbent_advantage: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyMode {
    pub description: String,
    pub dc_modifier: i32,
    pub ap_bonus: i32,
    pub npc_grudge_decay: f64,
    pub scandal_frequency: f64,
    pub economy_volatility: f64,
    pub allow_reload: bool,
}

impl GameConfig {
    /// Load from ~/.polit/config/
    pub fn load_from_home() -> Result<Self, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME").unwrap_or_default();
        let config_dir = format!("{}/.polit/config", home);
        Self::load(&config_dir)
    }

    /// Hardcoded defaults when no config files exist
    pub fn default_config() -> Self {
        Self {
            balance: BalanceConfig {
                action_points: ActionPointsConfig {
                    local: 5,
                    state: 7,
                    federal_house: 8,
                    federal_senate: 9,
                    governor: 9,
                    president: 12,
                    bureaucratic_low: 5,
                    bureaucratic_high: 8,
                },
                action_costs: ActionCostsConfig {
                    meet_in_person: 2,
                    phone_call: 1,
                    speech: 1,
                    campaign: 2,
                    draft_legislation: 0,
                    research: 1,
                    scheme: 2,
                    press_conference: 1,
                },
                dice: DiceConfig {
                    sides: 20,
                    crit_success: 20,
                    crit_failure: 1,
                },
                cards: CardsConfig {
                    max_deck_size: 30,
                    evolution_threshold: 10,
                    neglect_threshold: 20,
                },
                coherence: CoherenceConfig {
                    principled_threshold: 5,
                    flipflopper_threshold: -3,
                },
                stress: StressConfig {
                    crisis_stress: 10,
                    scandal_stress: 15,
                    overwork_threshold: 80,
                    burnout_threshold: 95,
                },
                elections: ElectionsConfig {
                    campaign_weeks_primary: 12,
                    campaign_weeks_general: 12,
                    incumbent_advantage: 5,
                },
            },
            difficulty: DifficultyMode {
                description: "Balanced challenge.".into(),
                dc_modifier: 0,
                ap_bonus: 0,
                npc_grudge_decay: 1.0,
                scandal_frequency: 1.0,
                economy_volatility: 1.0,
                allow_reload: false,
            },
        }
    }

    /// Load configuration from the game/config/ directory
    pub fn load(config_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Path::new(config_dir);

        // Load balance
        let balance_str = std::fs::read_to_string(config_path.join("balance.toml"))?;
        let balance: BalanceConfig = toml::from_str(&balance_str)?;

        // Load difficulty (default to standard)
        let diff_str = std::fs::read_to_string(config_path.join("difficulty.toml"))?;
        let diff_table: HashMap<String, DifficultyMode> = toml::from_str(&diff_str)?;
        let difficulty = diff_table
            .get("standard")
            .cloned()
            .ok_or("Missing 'standard' difficulty mode")?;

        Ok(Self {
            balance,
            difficulty,
        })
    }

    /// Load with a specific difficulty mode
    pub fn load_with_difficulty(
        config_dir: &str,
        mode: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Path::new(config_dir);

        let balance_str = std::fs::read_to_string(config_path.join("balance.toml"))?;
        let balance: BalanceConfig = toml::from_str(&balance_str)?;

        let diff_str = std::fs::read_to_string(config_path.join("difficulty.toml"))?;
        let diff_table: HashMap<String, DifficultyMode> = toml::from_str(&diff_str)?;
        let difficulty = diff_table.get(mode).cloned().ok_or_else(|| {
            format!(
                "Unknown difficulty mode: '{}'. Options: story, standard, ironman, nightmare",
                mode
            )
        })?;

        Ok(Self {
            balance,
            difficulty,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balance_config() {
        let config = GameConfig::load("game/config").unwrap();
        assert_eq!(config.balance.action_points.local, 5);
        assert_eq!(config.balance.action_points.president, 12);
        assert_eq!(config.balance.dice.sides, 20);
        assert_eq!(config.balance.cards.max_deck_size, 30);
    }

    #[test]
    fn test_load_difficulty_modes() {
        for mode in &["story", "standard", "ironman", "nightmare"] {
            let config = GameConfig::load_with_difficulty("game/config", mode).unwrap();
            assert!(!config.difficulty.description.is_empty());
        }
    }

    #[test]
    fn test_difficulty_scaling() {
        let story = GameConfig::load_with_difficulty("game/config", "story").unwrap();
        let nightmare = GameConfig::load_with_difficulty("game/config", "nightmare").unwrap();

        assert!(story.difficulty.dc_modifier < nightmare.difficulty.dc_modifier);
        assert!(story.difficulty.scandal_frequency < nightmare.difficulty.scandal_frequency);
        assert!(story.difficulty.allow_reload);
        assert!(!nightmare.difficulty.allow_reload);
    }

    #[test]
    fn test_invalid_difficulty_mode() {
        let result = GameConfig::load_with_difficulty("game/config", "impossible");
        assert!(result.is_err());
    }
}
