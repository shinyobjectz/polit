use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::engine::components::{Memory, Relationship, RelationshipType};

/// A character node in the social graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterNode {
    pub id: String,
    pub name: String,
    pub is_player: bool,
}

/// The social network graph
pub struct SocialGraph {
    graph: DiGraph<CharacterNode, Relationship>,
    /// Map from character ID to graph node index
    id_to_node: HashMap<String, NodeIndex>,
}

impl SocialGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_to_node: HashMap::new(),
        }
    }

    /// Add a character to the graph
    pub fn add_character(&mut self, id: &str, name: &str, is_player: bool) -> NodeIndex {
        if let Some(&idx) = self.id_to_node.get(id) {
            return idx;
        }
        let node = CharacterNode {
            id: id.to_string(),
            name: name.to_string(),
            is_player,
        };
        let idx = self.graph.add_node(node);
        self.id_to_node.insert(id.to_string(), idx);
        idx
    }

    /// Remove a character (retirement, death)
    pub fn remove_character(&mut self, id: &str) -> bool {
        if let Some(&idx) = self.id_to_node.get(id) {
            self.graph.remove_node(idx);
            self.id_to_node.remove(id);
            true
        } else {
            false
        }
    }

    /// Set or update a relationship between two characters
    pub fn set_relationship(&mut self, from_id: &str, to_id: &str, rel: Relationship) {
        let from = match self.id_to_node.get(from_id) {
            Some(&idx) => idx,
            None => return,
        };
        let to = match self.id_to_node.get(to_id) {
            Some(&idx) => idx,
            None => return,
        };

        // Update existing edge or add new one
        if let Some(edge) = self.graph.find_edge(from, to) {
            self.graph[edge] = rel;
        } else {
            self.graph.add_edge(from, to, rel);
        }
    }

    /// Get a relationship between two characters
    pub fn get_relationship(&self, from_id: &str, to_id: &str) -> Option<&Relationship> {
        let from = self.id_to_node.get(from_id)?;
        let to = self.id_to_node.get(to_id)?;
        let edge = self.graph.find_edge(*from, *to)?;
        Some(&self.graph[edge])
    }

    /// Modify a specific relationship field
    pub fn modify_relationship(
        &mut self,
        from_id: &str,
        to_id: &str,
        field: &str,
        delta: i32,
    ) -> bool {
        let from = match self.id_to_node.get(from_id) {
            Some(&idx) => idx,
            None => return false,
        };
        let to = match self.id_to_node.get(to_id) {
            Some(&idx) => idx,
            None => return false,
        };

        let edge = match self.graph.find_edge(from, to) {
            Some(e) => e,
            None => {
                // Create default relationship if none exists
                let e = self.graph.add_edge(from, to, Relationship::default());
                e
            }
        };

        let rel = &mut self.graph[edge];
        match field {
            "trust" => rel.trust = (rel.trust + delta).clamp(-100, 100),
            "respect" => rel.respect = (rel.respect + delta).clamp(-100, 100),
            "fear" => rel.fear = (rel.fear + delta).clamp(0, 100),
            "loyalty" => rel.loyalty = (rel.loyalty + delta).clamp(0, 100),
            "debt" => rel.debt = (rel.debt + delta).clamp(-10, 10),
            "knowledge" => rel.knowledge = (rel.knowledge + delta).clamp(0, 100),
            "leverage" => rel.leverage = (rel.leverage + delta).clamp(0, 100),
            _ => return false,
        }
        true
    }

    /// Add a memory to a relationship
    pub fn add_memory(
        &mut self,
        from_id: &str,
        to_id: &str,
        week: u32,
        description: &str,
        impact: i32,
    ) {
        let from = match self.id_to_node.get(from_id) {
            Some(&idx) => idx,
            None => return,
        };
        let to = match self.id_to_node.get(to_id) {
            Some(&idx) => idx,
            None => return,
        };

        let edge = match self.graph.find_edge(from, to) {
            Some(e) => e,
            None => self.graph.add_edge(from, to, Relationship::default()),
        };

        self.graph[edge].memories.push(Memory {
            week,
            description: description.to_string(),
            impact,
        });
    }

    /// Get all relationships for a character
    pub fn get_all_relationships(&self, id: &str) -> Vec<(&str, &Relationship)> {
        let node = match self.id_to_node.get(id) {
            Some(&idx) => idx,
            None => return vec![],
        };

        self.graph
            .edges(node)
            .map(|edge| {
                let target = edge.target();
                let name = self.graph[target].id.as_str();
                (name, edge.weight())
            })
            .collect()
    }

    /// Get allies (trust > threshold)
    pub fn get_allies(&self, id: &str, trust_threshold: i32) -> Vec<(&str, &Relationship)> {
        self.get_all_relationships(id)
            .into_iter()
            .filter(|(_, rel)| rel.trust >= trust_threshold)
            .collect()
    }

    /// Get rivals (trust < negative threshold)
    pub fn get_rivals(&self, id: &str, trust_threshold: i32) -> Vec<(&str, &Relationship)> {
        self.get_all_relationships(id)
            .into_iter()
            .filter(|(_, rel)| rel.trust <= -trust_threshold)
            .collect()
    }

    /// Propagate reputation change through the network
    /// When character A does something to B, B's allies hear about it
    pub fn propagate_reputation(
        &mut self,
        subject_id: &str,
        field: &str,
        base_delta: i32,
        max_hops: usize,
    ) -> Vec<(String, i32)> {
        let mut affected = Vec::new();
        let subject_node = match self.id_to_node.get(subject_id) {
            Some(&idx) => idx,
            None => return affected,
        };

        // Get all nodes connected to the subject
        let direct_targets: Vec<(NodeIndex, i32)> = self
            .graph
            .edges(subject_node)
            .map(|e| (e.target(), self.graph[e.id()].loyalty))
            .collect();

        // First hop: direct connections with loyalty > 60 hear about it
        for (target, loyalty) in &direct_targets {
            if *loyalty > 60 {
                let attenuation = 0.5; // 50% of original impact
                let propagated_delta = (base_delta as f64 * attenuation) as i32;
                if propagated_delta != 0 {
                    let target_id = self.graph[*target].id.clone();
                    affected.push((target_id, propagated_delta));
                }
            }
        }

        // Apply propagated changes
        for (target_id, delta) in &affected {
            self.modify_relationship(subject_id, target_id, field, *delta);
        }

        affected
    }

    /// Get character name by ID
    pub fn get_name(&self, id: &str) -> Option<&str> {
        self.id_to_node
            .get(id)
            .map(|&idx| self.graph[idx].name.as_str())
    }

    /// Total number of characters
    pub fn character_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Total number of relationships
    pub fn relationship_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_graph() -> SocialGraph {
        let mut g = SocialGraph::new();
        g.add_character("player", "Alex Rivera", true);
        g.add_character("davis", "Councilwoman Davis", false);
        g.add_character("kowalski", "Chief Kowalski", false);
        g.add_character("martinez", "Sen. Martinez", false);
        g.add_character("kim", "Journalist Kim", false);

        g.set_relationship(
            "player",
            "davis",
            Relationship {
                trust: -20,
                respect: 30,
                loyalty: 10,
                ..Relationship::default()
            },
        );
        g.set_relationship(
            "player",
            "kowalski",
            Relationship {
                trust: 45,
                respect: 60,
                loyalty: 50,
                ..Relationship::default()
            },
        );
        g.set_relationship(
            "player",
            "martinez",
            Relationship {
                trust: 70,
                respect: 55,
                loyalty: 65,
                ..Relationship::default()
            },
        );
        g.set_relationship(
            "davis",
            "martinez",
            Relationship {
                trust: 60,
                respect: 50,
                loyalty: 70,
                ..Relationship::default()
            },
        );
        g.set_relationship(
            "martinez",
            "kim",
            Relationship {
                trust: 40,
                respect: 45,
                loyalty: 30,
                ..Relationship::default()
            },
        );

        g
    }

    #[test]
    fn test_add_characters() {
        let g = setup_graph();
        assert_eq!(g.character_count(), 5);
        assert_eq!(g.get_name("davis"), Some("Councilwoman Davis"));
    }

    #[test]
    fn test_get_relationship() {
        let g = setup_graph();
        let rel = g.get_relationship("player", "davis").unwrap();
        assert_eq!(rel.trust, -20);
        assert_eq!(rel.respect, 30);
    }

    #[test]
    fn test_modify_relationship() {
        let mut g = setup_graph();
        g.modify_relationship("player", "davis", "trust", 15);
        let rel = g.get_relationship("player", "davis").unwrap();
        assert_eq!(rel.trust, -5); // -20 + 15
    }

    #[test]
    fn test_clamping() {
        let mut g = setup_graph();
        g.modify_relationship("player", "davis", "trust", 200);
        let rel = g.get_relationship("player", "davis").unwrap();
        assert_eq!(rel.trust, 100); // clamped

        g.modify_relationship("player", "davis", "fear", -50);
        let rel = g.get_relationship("player", "davis").unwrap();
        assert_eq!(rel.fear, 0); // clamped at 0
    }

    #[test]
    fn test_add_memory() {
        let mut g = setup_graph();
        g.add_memory("player", "davis", 5, "Blocked zoning proposal", -10);
        let rel = g.get_relationship("player", "davis").unwrap();
        assert_eq!(rel.memories.len(), 1);
        assert_eq!(rel.memories[0].description, "Blocked zoning proposal");
    }

    #[test]
    fn test_get_allies() {
        let g = setup_graph();
        let allies = g.get_allies("player", 40);
        assert_eq!(allies.len(), 2); // kowalski (45) and martinez (70)
    }

    #[test]
    fn test_get_rivals() {
        let g = setup_graph();
        let rivals = g.get_rivals("player", 10);
        assert_eq!(rivals.len(), 1); // davis (-20)
    }

    #[test]
    fn test_relationship_auto_create() {
        let mut g = setup_graph();
        // Modify relationship that doesn't exist yet → creates default
        g.modify_relationship("player", "kim", "trust", 25);
        let rel = g.get_relationship("player", "kim").unwrap();
        assert_eq!(rel.trust, 25);
    }

    #[test]
    fn test_remove_character() {
        let mut g = setup_graph();
        assert!(g.remove_character("davis"));
        assert_eq!(g.character_count(), 4);
        assert!(g.get_relationship("player", "davis").is_none());
    }

    #[test]
    fn test_propagation() {
        let mut g = setup_graph();
        // Martinez has loyalty 65 to player → should propagate
        let affected = g.propagate_reputation("player", "trust", -20, 1);
        // Martinez (loyalty 65 > 60) should be affected
        assert!(!affected.is_empty());
    }

    #[test]
    fn test_all_relationships() {
        let g = setup_graph();
        let rels = g.get_all_relationships("player");
        assert_eq!(rels.len(), 3); // davis, kowalski, martinez
    }

    #[test]
    fn test_duplicate_add() {
        let mut g = SocialGraph::new();
        let idx1 = g.add_character("test", "Test", false);
        let idx2 = g.add_character("test", "Test", false);
        assert_eq!(idx1, idx2); // Same node
        assert_eq!(g.character_count(), 1);
    }
}
