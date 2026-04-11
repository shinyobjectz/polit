use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

// ===== CHARACTER COMPONENTS =====

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub age: u32,
    pub gender: Gender,
    pub background: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Nonbinary,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PoliticalRole {
    pub office: Option<Office>,
    pub party: Option<Party>,
    pub faction: Option<String>,
    pub committees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Office {
    CityCouncil,
    Mayor,
    SchoolBoard,
    CountyClerk,
    StateLegislator,
    StateAG,
    Governor,
    USHouse,
    USSenate,
    President,
    VicePresident,
    SupremeCourtJustice,
    CabinetSecretary(String),
    AgencyHead(String),
    FederalJudge,
    // Bureaucratic
    CivilServant { grade: u8 },
    MilitaryOfficer { rank: String },
    LawEnforcement { rank: String },
    IntelligenceOfficer { rank: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Party {
    Democrat,
    Republican,
    Independent,
    Custom(String),
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    /// Big Five traits, each 0.0 - 1.0
    pub openness: f32,
    pub conscientiousness: f32,
    pub extraversion: f32,
    pub agreeableness: f32,
    pub neuroticism: f32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Ideology {
    /// Each axis: 0.0 (left/progressive) to 1.0 (right/conservative)
    pub economic: f32,
    pub social: f32,
    pub foreign_policy: f32, // 0.0 = dove, 1.0 = hawk
    pub governance: f32,     // 0.0 = big gov, 1.0 = small gov
    pub environment: f32,    // 0.0 = green, 1.0 = industry
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub persuasion: i32,
    pub cunning: i32,
    pub charisma: i32,
    pub knowledge: i32,
    pub ruthlessness: i32,
    pub loyalty: i32,
    pub media_savvy: i32,
    pub endurance: i32,
    pub discretion: i32,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub stress: i32,   // 0-100
    pub physical: i32, // 0-100
    pub burnout: bool,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Goals {
    pub short_term: Vec<String>,
    pub long_term: Vec<String>,
}

/// Marker for the player entity
#[derive(Component, Debug)]
pub struct Player;

/// Marker for NPC entities
#[derive(Component, Debug)]
pub struct Npc;

// ===== CARD COMPONENTS =====

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub card_type: CardType,
    pub rarity: Rarity,
    pub description: String,
    pub ap_cost: i32,
    pub requirements: Vec<String>,
    pub effects: Vec<CardEffect>,
    pub play_count: u32,
    pub neglect_counter: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardType {
    Tactic(TacticCategory),
    Asset(AssetCategory),
    Position(PositionCategory),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TacticCategory {
    Political,
    Media,
    Campaign,
    Covert,
    Legal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetCategory {
    People,
    Organization,
    Resource,
    Institutional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionCategory {
    Economic,
    Social,
    ForeignPolicy,
    Governance,
    WedgeIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardEffect {
    ModifyStat {
        stat: String,
        delta: i32,
    },
    ModifyRoll {
        skill: String,
        bonus: i32,
    },
    ModifyRelationship {
        target: String,
        field: String,
        delta: i32,
    },
    ModifyEconomic {
        variable: String,
        delta: f64,
    },
    GrantCard {
        card_id: String,
    },
    TriggerEvent {
        event_id: String,
    },
    Custom {
        rhai_script: String,
    },
}

// ===== LAW COMPONENTS =====

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Law {
    pub id: String,
    pub title: String,
    pub jurisdiction: Jurisdiction,
    pub law_type: LawType,
    pub sponsor_id: Option<String>,
    pub player_draft: String,
    pub legal_text: String,
    pub plain_summary: String,
    pub stage: LawStage,
    pub votes_for: u32,
    pub votes_against: u32,
    pub enacted_week: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Jurisdiction {
    Federal,
    State(String),
    Local(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LawType {
    Statute,
    ExecutiveOrder,
    Regulation,
    Amendment,
    Ordinance,
    Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LawStage {
    Draft,
    Committee,
    Floor,
    Enacted,
    StruckDown,
    Expired,
}

// ===== INFORMATION COMPONENTS =====

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Information {
    pub id: String,
    pub info_type: InfoType,
    pub topic: String,
    pub about: String,
    pub truth_value: f32,   // 0.0-1.0
    pub severity: u8,       // 1-10
    pub newsworthiness: u8, // 1-10
    pub evidence_level: EvidenceLevel,
    pub public: bool,
    pub public_belief: f32, // 0.0-1.0
    pub created_week: u32,
    pub published_week: Option<u32>,
    pub status: InfoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfoType {
    Fact,
    Rumor,
    Leak,
    Spin,
    Fabrication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceLevel {
    None,
    Circumstantial,
    Documented,
    Recorded,
    Undeniable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InfoStatus {
    Secret,
    Rumored,
    Reported,
    Confirmed,
    OldNews,
    Forgotten,
}

// ===== ECONOMIC COMPONENTS =====

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct EconomicState {
    pub gdp: f64,
    pub gdp_growth: f64,
    pub unemployment: f64,
    pub inflation: f64,
    pub federal_funds_rate: f64,
    pub national_debt: f64,
    pub trade_balance: f64,
    pub consumer_confidence: f64,
    pub gini_coefficient: f64,
}

// ===== RELATIONSHIP (edge data, stored in social graph not as ECS component) =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub trust: i32,     // -100 to 100
    pub respect: i32,   // -100 to 100
    pub fear: i32,      // 0 to 100
    pub loyalty: i32,   // 0 to 100
    pub debt: i32,      // -10 to 10
    pub knowledge: i32, // 0 to 100
    pub leverage: i32,  // 0 to 100
    pub rel_type: RelationshipType,
    pub memories: Vec<Memory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Ally,
    Rival,
    Mentor,
    Protege,
    Neutral,
    Enemy,
    Family,
    Donor,
    Staffer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub week: u32,
    pub description: String,
    pub impact: i32,
}

impl Default for Relationship {
    fn default() -> Self {
        Self {
            trust: 0,
            respect: 0,
            fear: 0,
            loyalty: 0,
            debt: 0,
            knowledge: 0,
            leverage: 0,
            rel_type: RelationshipType::Neutral,
            memories: Vec::new(),
        }
    }
}
