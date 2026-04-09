use bevy_ecs::prelude::*;

use super::components::*;
use super::GameClock;

/// Build the initial ECS world with global resources
pub fn build_world() -> World {
    let mut world = World::new();

    world.insert_resource(GameClock { week: 1, year: 2024 });

    world
}

/// Spawn a player character entity
pub fn spawn_player(world: &mut World, name: &str) -> Entity {
    world.spawn((
        Player,
        Identity {
            name: name.to_string(),
            age: 35,
            gender: Gender::Male,
            background: "Newcomer to politics".to_string(),
        },
        PoliticalRole {
            office: Some(Office::CityCouncil),
            party: Some(Party::Democrat),
            faction: None,
            committees: vec![],
        },
        Personality {
            openness: 0.7,
            conscientiousness: 0.6,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.3,
        },
        Ideology {
            economic: 0.4,
            social: 0.3,
            foreign_policy: 0.4,
            governance: 0.4,
            environment: 0.3,
        },
        Stats {
            persuasion: 10,
            cunning: 8,
            charisma: 12,
            knowledge: 10,
            ruthlessness: 5,
            loyalty: 8,
            media_savvy: 7,
            endurance: 10,
            discretion: 8,
        },
        Health {
            stress: 10,
            physical: 90,
            burnout: false,
        },
        Goals {
            short_term: vec!["Pass first ordinance".to_string()],
            long_term: vec!["Become mayor".to_string()],
        },
    )).id()
}

/// Spawn an NPC entity
pub fn spawn_npc(world: &mut World, name: &str, office: Option<Office>) -> Entity {
    world.spawn((
        Npc,
        Identity {
            name: name.to_string(),
            age: 50,
            gender: Gender::Female,
            background: "Career politician".to_string(),
        },
        PoliticalRole {
            office,
            party: Some(Party::Republican),
            faction: None,
            committees: vec![],
        },
        Personality {
            openness: 0.4,
            conscientiousness: 0.7,
            extraversion: 0.6,
            agreeableness: 0.3,
            neuroticism: 0.4,
        },
        Ideology {
            economic: 0.7,
            social: 0.6,
            foreign_policy: 0.6,
            governance: 0.7,
            environment: 0.6,
        },
        Stats {
            persuasion: 12,
            cunning: 14,
            charisma: 10,
            knowledge: 11,
            ruthlessness: 8,
            loyalty: 6,
            media_savvy: 9,
            endurance: 8,
            discretion: 7,
        },
        Health {
            stress: 20,
            physical: 75,
            burnout: false,
        },
        Goals {
            short_term: vec!["Block zoning reform".to_string()],
            long_term: vec!["Win mayoral race".to_string()],
        },
    )).id()
}
