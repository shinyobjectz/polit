use serde_json::json;

use polit::systems::election_inputs::ElectionInputs;

#[test]
fn election_inputs_deserialize_python_payload() {
    let payload = json!({
        "ideology_distribution": {
            "39049": { "left": 0.42, "center": 0.21, "right": 0.37 }
        },
        "turnout_propensity": { "39049": 0.63 },
        "economic_conditions": {
            "39049": { "unemployment": 0.054, "income_change": -120.0, "anxiety": 0.32 }
        },
        "approval_rating": { "39049": 44.5 },
        "issue_salience": { "economy": 0.81, "jobs": 0.74, "inflation": 0.51 },
        "swing_counties": ["39049"],
        "enthusiasm_gap": -0.18
    });

    let parsed: ElectionInputs = serde_json::from_value(payload).unwrap();

    assert_eq!(parsed.swing_counties, vec!["39049"]);
    assert_eq!(parsed.turnout_propensity["39049"], 0.63);
    assert_eq!(parsed.issue_salience["economy"], 0.81);
    assert_eq!(parsed.economic_conditions["39049"].anxiety, 0.32);
}

#[test]
fn election_inputs_msgpack_roundtrip() {
    let payload = json!({
        "ideology_distribution": {
            "39049": { "left": 0.42, "center": 0.21, "right": 0.37 }
        },
        "turnout_propensity": { "39049": 0.63 },
        "economic_conditions": {
            "39049": { "unemployment": 0.054, "income_change": -120.0, "anxiety": 0.32 }
        },
        "approval_rating": { "39049": 44.5 },
        "issue_salience": { "economy": 0.81, "jobs": 0.74, "inflation": 0.51 },
        "swing_counties": ["39049"],
        "enthusiasm_gap": -0.18
    });
    let parsed: ElectionInputs = serde_json::from_value(payload).unwrap();

    let bytes = rmp_serde::to_vec(&parsed).unwrap();
    let restored: ElectionInputs = rmp_serde::from_slice(&bytes).unwrap();

    assert_eq!(restored, parsed);
}
