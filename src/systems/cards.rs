use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A card definition loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDef {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub category: String,
    pub rarity: String,
    pub ap_cost: i32,
    pub description: String,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub aligned_with: Vec<String>,
    #[serde(default)]
    pub contradicts: Vec<String>,
}

/// Container for loading cards from TOML
#[derive(Debug, Deserialize)]
pub struct CardFile {
    pub cards: Vec<CardDef>,
}

/// A card instance in the player's deck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInstance {
    pub def_id: String,
    pub play_count: u32,
    pub neglect_weeks: u32,
    pub evolved: bool,
}

/// The player's deck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    pub cards: Vec<CardInstance>,
    pub max_size: u32,
}

/// Coherence analysis result
#[derive(Debug, Clone)]
pub struct CoherenceResult {
    pub score: i32,
    pub aligned_pairs: Vec<(String, String)>,
    pub contradictions: Vec<(String, String)>,
    pub label: CoherenceLabel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoherenceLabel {
    Principled,  // score > threshold
    Pragmatist,  // score between thresholds
    FlipFlopper, // score < negative threshold
}

impl Deck {
    pub fn new(max_size: u32) -> Self {
        Self {
            cards: Vec::new(),
            max_size,
        }
    }

    /// Add a card to the deck. Returns false if deck is full.
    pub fn add_card(&mut self, def_id: &str) -> bool {
        if self.cards.len() as u32 >= self.max_size {
            return false;
        }
        self.cards.push(CardInstance {
            def_id: def_id.to_string(),
            play_count: 0,
            neglect_weeks: 0,
            evolved: false,
        });
        true
    }

    /// Remove a card by definition ID. Returns true if found.
    pub fn remove_card(&mut self, def_id: &str) -> bool {
        if let Some(idx) = self.cards.iter().position(|c| c.def_id == def_id) {
            self.cards.remove(idx);
            true
        } else {
            false
        }
    }

    /// Check if the deck contains a card
    pub fn has_card(&self, def_id: &str) -> bool {
        self.cards.iter().any(|c| c.def_id == def_id)
    }

    /// Mark a card as played (increment play count, reset neglect)
    pub fn play_card(&mut self, def_id: &str) -> bool {
        if let Some(card) = self.cards.iter_mut().find(|c| c.def_id == def_id) {
            card.play_count += 1;
            card.neglect_weeks = 0;
            true
        } else {
            false
        }
    }

    /// Increment neglect counters for all asset cards (called each week)
    pub fn tick_neglect(&mut self) {
        for card in &mut self.cards {
            card.neglect_weeks += 1;
        }
    }

    /// Get cards by type
    pub fn tactics(&self) -> Vec<&CardInstance> {
        // Would need card registry to filter by type — for now return all
        self.cards.iter().collect()
    }

    pub fn card_count(&self) -> usize {
        self.cards.len()
    }

    /// Calculate coherence score from position cards
    pub fn calculate_coherence(&self, card_defs: &HashMap<String, CardDef>) -> CoherenceResult {
        let positions: Vec<&CardDef> = self
            .cards
            .iter()
            .filter_map(|c| card_defs.get(&c.def_id))
            .filter(|d| d.card_type == "position")
            .collect();

        let mut aligned_pairs = Vec::new();
        let mut contradictions = Vec::new();

        for (i, a) in positions.iter().enumerate() {
            for b in positions.iter().skip(i + 1) {
                if a.aligned_with.contains(&b.id) || b.aligned_with.contains(&a.id) {
                    aligned_pairs.push((a.id.clone(), b.id.clone()));
                }
                if a.contradicts.contains(&b.id) || b.contradicts.contains(&a.id) {
                    contradictions.push((a.id.clone(), b.id.clone()));
                }
            }
        }

        let score = aligned_pairs.len() as i32 - contradictions.len() as i32;

        let label = if score > 5 {
            CoherenceLabel::Principled
        } else if score < -3 {
            CoherenceLabel::FlipFlopper
        } else {
            CoherenceLabel::Pragmatist
        };

        CoherenceResult {
            score,
            aligned_pairs,
            contradictions,
            label,
        }
    }
}

/// Load card definitions from a TOML string
pub fn load_cards_from_toml(toml_str: &str) -> Result<Vec<CardDef>, toml::de::Error> {
    let file: CardFile = toml::from_str(toml_str)?;
    Ok(file.cards)
}

/// Build a card registry from multiple card definition files
pub fn build_card_registry(card_lists: Vec<Vec<CardDef>>) -> HashMap<String, CardDef> {
    let mut registry = HashMap::new();
    for list in card_lists {
        for card in list {
            registry.insert(card.id.clone(), card);
        }
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> HashMap<String, CardDef> {
        let toml_str = r#"
[[cards]]
id = "free_trade"
name = "Free Trade"
type = "position"
category = "economic"
rarity = "common"
ap_cost = 0
description = "Open markets"
aligned_with = ["pro_business"]
contradicts = ["protectionist"]

[[cards]]
id = "pro_business"
name = "Pro-Business"
type = "position"
category = "economic"
rarity = "common"
ap_cost = 0
description = "Business friendly"
aligned_with = ["free_trade"]
contradicts = []

[[cards]]
id = "protectionist"
name = "Protectionist"
type = "position"
category = "economic"
rarity = "common"
ap_cost = 0
description = "Tariffs"
aligned_with = []
contradicts = ["free_trade"]

[[cards]]
id = "stump_speech"
name = "Stump Speech"
type = "tactic"
category = "campaign"
rarity = "common"
ap_cost = 1
description = "Basic speech"
requirements = []
"#;
        let cards = load_cards_from_toml(toml_str).unwrap();
        build_card_registry(vec![cards])
    }

    #[test]
    fn test_load_cards() {
        let reg = sample_registry();
        assert_eq!(reg.len(), 4);
        assert!(reg.contains_key("free_trade"));
        assert_eq!(reg["stump_speech"].card_type, "tactic");
    }

    #[test]
    fn test_deck_add_remove() {
        let mut deck = Deck::new(30);
        assert!(deck.add_card("stump_speech"));
        assert_eq!(deck.card_count(), 1);
        assert!(deck.has_card("stump_speech"));

        assert!(deck.remove_card("stump_speech"));
        assert_eq!(deck.card_count(), 0);
        assert!(!deck.has_card("stump_speech"));
    }

    #[test]
    fn test_deck_max_size() {
        let mut deck = Deck::new(3);
        assert!(deck.add_card("a"));
        assert!(deck.add_card("b"));
        assert!(deck.add_card("c"));
        assert!(!deck.add_card("d")); // Full
        assert_eq!(deck.card_count(), 3);
    }

    #[test]
    fn test_card_play_count() {
        let mut deck = Deck::new(30);
        deck.add_card("stump_speech");
        deck.play_card("stump_speech");
        deck.play_card("stump_speech");
        assert_eq!(deck.cards[0].play_count, 2);
        assert_eq!(deck.cards[0].neglect_weeks, 0);
    }

    #[test]
    fn test_neglect_tracking() {
        let mut deck = Deck::new(30);
        deck.add_card("test");
        deck.tick_neglect();
        deck.tick_neglect();
        assert_eq!(deck.cards[0].neglect_weeks, 2);

        deck.play_card("test");
        assert_eq!(deck.cards[0].neglect_weeks, 0);
    }

    #[test]
    fn test_coherence_aligned() {
        let reg = sample_registry();
        let mut deck = Deck::new(30);
        deck.add_card("free_trade");
        deck.add_card("pro_business");

        let result = deck.calculate_coherence(&reg);
        assert_eq!(result.aligned_pairs.len(), 1);
        assert_eq!(result.contradictions.len(), 0);
        assert!(result.score > 0);
    }

    #[test]
    fn test_coherence_contradictory() {
        let reg = sample_registry();
        let mut deck = Deck::new(30);
        deck.add_card("free_trade");
        deck.add_card("protectionist");

        let result = deck.calculate_coherence(&reg);
        assert_eq!(result.contradictions.len(), 1);
        assert!(result.score < 0);
    }

    #[test]
    fn test_coherence_labels() {
        let reg = sample_registry();

        // Pragmatist (no positions)
        let deck = Deck::new(30);
        let result = deck.calculate_coherence(&reg);
        assert_eq!(result.label, CoherenceLabel::Pragmatist);
    }

    #[test]
    fn test_load_real_starter_deck() {
        let toml_str = include_str!("../../game/scenarios/modern_usa/cards/starter_idealist.toml");
        let cards = load_cards_from_toml(toml_str).unwrap();
        assert!(cards.len() >= 8, "Starter deck should have 8+ cards");

        let positions: Vec<_> = cards.iter().filter(|c| c.card_type == "position").collect();
        assert!(
            positions.len() >= 3,
            "Idealist should have 3+ position cards"
        );
    }

    #[test]
    fn test_load_acquirable_cards() {
        let toml_str = include_str!("../../game/scenarios/modern_usa/cards/acquirable.toml");
        let cards = load_cards_from_toml(toml_str).unwrap();
        assert!(
            cards.len() >= 15,
            "Should have 15+ acquirable cards, got {}",
            cards.len()
        );

        // Check all three types present
        let tactics: Vec<_> = cards.iter().filter(|c| c.card_type == "tactic").collect();
        let assets: Vec<_> = cards.iter().filter(|c| c.card_type == "asset").collect();
        let positions: Vec<_> = cards.iter().filter(|c| c.card_type == "position").collect();
        assert!(!tactics.is_empty());
        assert!(!assets.is_empty());
        assert!(!positions.is_empty());
    }
}
