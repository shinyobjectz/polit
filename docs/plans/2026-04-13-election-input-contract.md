# Election Input Contract Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a Rust-side simulation election input contract that matches the Python `sim.layers.elections` payload shape.

**Architecture:** Keep the work at the bridge boundary only. Define a serializable Rust model for election inputs, verify it against the current Python payload shape with failing tests first, and avoid wiring any gameplay vote logic in this pass.

**Tech Stack:** Rust 2021, `serde`, `serde_json`, `rmp-serde`, existing `polit::systems` module layout

---

### Task 1: Rust Election Input Contract

**Files:**
- Create: `src/systems/election_inputs.rs`
- Modify: `src/systems/mod.rs`
- Test: `tests/election_inputs.rs`

**Step 1: Write the failing test**

```rust
use polit::systems::election_inputs::ElectionInputs;

#[test]
fn election_inputs_deserialize_python_payload() {
    let payload = serde_json::json!({
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
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test election_inputs`
Expected: FAIL with unresolved import for `polit::systems::election_inputs`

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdeologyDistribution {
    pub left: f64,
    pub center: f64,
    pub right: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EconomicConditions {
    pub unemployment: f64,
    pub income_change: f64,
    pub anxiety: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElectionInputs {
    pub ideology_distribution: HashMap<String, IdeologyDistribution>,
    pub turnout_propensity: HashMap<String, f64>,
    pub economic_conditions: HashMap<String, EconomicConditions>,
    pub approval_rating: HashMap<String, f64>,
    pub issue_salience: HashMap<String, f64>,
    pub swing_counties: Vec<String>,
    pub enthusiasm_gap: f64,
}
```

**Step 4: Add round-trip coverage**

```rust
#[test]
fn election_inputs_msgpack_roundtrip() {
    let mut inputs = ElectionInputs::default();
    inputs.swing_counties.push("39049".into());
    let bytes = rmp_serde::to_vec(&inputs).unwrap();
    let restored: ElectionInputs = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(restored.swing_counties, vec!["39049"]);
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test --test election_inputs`
Expected: PASS

**Step 6: Run broader verification**

Run: `make test`
Expected: PASS

Run: `make lint`
Expected: PASS

**Step 7: Commit**

```bash
git add docs/plans/2026-04-13-election-input-contract.md tests/election_inputs.rs src/systems/election_inputs.rs src/systems/mod.rs
git commit -m "feat(sim): add Rust election input contract"
```
