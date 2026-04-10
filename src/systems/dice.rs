use rand::Rng;
use serde::{Deserialize, Serialize};

/// Result of a D20 skill check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResult {
    pub natural_roll: u32,
    pub modifiers: i32,
    pub total: i32,
    pub dc: u32,
    pub success: bool,
    pub critical_success: bool,
    pub critical_failure: bool,
    pub skill: String,
}

/// Roll a D20 with modifiers against a difficulty class
pub fn skill_check(skill: &str, modifier: i32, dc: u32) -> RollResult {
    let mut rng = rand::thread_rng();
    let natural_roll = rng.gen_range(1..=20);
    let total = natural_roll as i32 + modifier;
    let success = total >= dc as i32;
    let critical_success = natural_roll == 20;
    let critical_failure = natural_roll == 1;

    RollResult {
        natural_roll,
        modifiers: modifier,
        total,
        dc,
        // Nat 20 always succeeds, nat 1 always fails
        success: if critical_success {
            true
        } else if critical_failure {
            false
        } else {
            success
        },
        critical_success,
        critical_failure,
        skill: skill.to_string(),
    }
}

/// Roll a plain die (for non-skill situations)
pub fn roll(sides: u32) -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=sides)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_bounds() {
        for _ in 0..1000 {
            let r = roll(20);
            assert!(r >= 1 && r <= 20);
        }
    }

    #[test]
    fn test_skill_check_success() {
        // With +20 modifier against DC 10, should almost always succeed
        let mut successes = 0;
        for _ in 0..100 {
            let result = skill_check("Persuasion", 20, 10);
            if result.success {
                successes += 1;
            }
        }
        // Even nat 1 fails, so ~95% success rate
        assert!(successes > 90);
    }

    #[test]
    fn test_critical_detection() {
        // Run enough rolls to hit both crits
        let mut found_crit_success = false;
        let mut found_crit_failure = false;
        for _ in 0..10000 {
            let result = skill_check("test", 0, 10);
            if result.critical_success {
                found_crit_success = true;
                assert!(result.success);
            }
            if result.critical_failure {
                found_crit_failure = true;
                assert!(!result.success);
            }
            if found_crit_success && found_crit_failure {
                break;
            }
        }
        assert!(found_crit_success, "Never rolled a nat 20 in 10000 rolls");
        assert!(found_crit_failure, "Never rolled a nat 1 in 10000 rolls");
    }

    #[test]
    fn test_modifier_applied() {
        let result = skill_check("test", 5, 10);
        assert_eq!(result.total, result.natural_roll as i32 + 5);
    }

    #[test]
    fn test_distribution_roughly_uniform() {
        let mut counts = [0u32; 20];
        let n = 20000;
        for _ in 0..n {
            let r = roll(20);
            counts[(r - 1) as usize] += 1;
        }
        let expected = n / 20;
        for (i, &count) in counts.iter().enumerate() {
            let deviation = (count as f64 - expected as f64).abs() / expected as f64;
            assert!(
                deviation < 0.15,
                "Roll {} appeared {} times, expected ~{}, deviation {:.2}%",
                i + 1,
                count,
                expected,
                deviation * 100.0
            );
        }
    }
}
