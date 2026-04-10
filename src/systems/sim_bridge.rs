use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::collections::HashMap;

use crate::systems::sim_events::SimEvent;
use crate::systems::world_delta::WorldStateDelta;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MacroSnapshot {
    pub gdp_growth: f64,
    pub inflation: f64,
    pub unemployment: f64,
    pub fed_funds_rate: f64,
    pub consumer_confidence: f64,
    pub debt_to_gdp: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct WorldStateSnapshot {
    pub week: u32,
    pub macro_state: MacroSnapshot,
    pub counties: HashMap<String, serde_json::Value>,
}

pub struct SimBridge {
    // No persistent state needed — Python GIL acquired per-call
}

impl SimBridge {
    pub fn new() -> Result<Self, SimBridgeError> {
        Python::with_gil(|py| {
            // Add project root (parent of sim/) to Python path so `import sim.host` works
            let sim_path = std::env::current_dir()
                .unwrap_or_default()
                .join("sim");
            let project_root = sim_path
                .parent()
                .unwrap_or(&sim_path);
            let sys = py
                .import("sys")
                .map_err(|e| SimBridgeError::PythonInit(e.to_string()))?;
            let path = sys
                .getattr("path")
                .map_err(|e| SimBridgeError::PythonInit(e.to_string()))?;
            path.call_method1(
                "insert",
                (0, project_root.to_str().unwrap_or(".")),
            )
            .map_err(|e| SimBridgeError::PythonInit(e.to_string()))?;

            // Verify sim.host is importable
            py.import("sim.host")
                .map_err(|e| SimBridgeError::PythonImport(e.to_string()))?;
            Ok(SimBridge {})
        })
    }

    pub fn tick(
        &self,
        world_state: &WorldStateSnapshot,
        events: &[SimEvent],
    ) -> Result<WorldStateDelta, SimBridgeError> {
        let ws_bytes = rmp_serde::to_vec(world_state)
            .map_err(|e| SimBridgeError::Serialize(e.to_string()))?;
        let ev_bytes = rmp_serde::to_vec(events)
            .map_err(|e| SimBridgeError::Serialize(e.to_string()))?;

        Python::with_gil(|py| {
            let host = py
                .import("sim.host")
                .map_err(|e| SimBridgeError::PythonImport(e.to_string()))?;

            let ws_py = PyBytes::new(py, &ws_bytes);
            let ev_py = PyBytes::new(py, &ev_bytes);

            let result = host
                .call_method1("tick", (ws_py, ev_py))
                .map_err(|e| SimBridgeError::PythonCall(e.to_string()))?;

            let result_bytes: &[u8] = result
                .downcast::<PyBytes>()
                .map_err(|e| SimBridgeError::PythonReturn(e.to_string()))?
                .as_bytes();

            rmp_serde::from_slice(result_bytes)
                .map_err(|e| SimBridgeError::Deserialize(e.to_string()))
        })
    }
}

#[derive(Debug)]
pub enum SimBridgeError {
    PythonInit(String),
    PythonImport(String),
    PythonCall(String),
    PythonReturn(String),
    Serialize(String),
    Deserialize(String),
}

impl std::fmt::Display for SimBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PythonInit(e) => write!(f, "Python initialization failed: {e}"),
            Self::PythonImport(e) => write!(f, "Failed to import sim.host: {e}"),
            Self::PythonCall(e) => write!(f, "Python tick() call failed: {e}"),
            Self::PythonReturn(e) => write!(f, "Python returned unexpected type: {e}"),
            Self::Serialize(e) => write!(f, "Serialization error: {e}"),
            Self::Deserialize(e) => write!(f, "Deserialization error: {e}"),
        }
    }
}

impl std::error::Error for SimBridgeError {}

#[cfg(test)]
#[cfg(feature = "simulation")]
mod tests {
    use super::*;
    use crate::systems::sim_events::{Sector, SimEvent};

    #[test]
    fn bridge_calls_python_and_returns_delta() {
        let bridge = SimBridge::new().expect("Failed to init Python");
        let world_state = WorldStateSnapshot {
            week: 1,
            macro_state: MacroSnapshot {
                gdp_growth: 0.02,
                inflation: 0.03,
                unemployment: 0.04,
                fed_funds_rate: 0.05,
                consumer_confidence: 100.0,
                debt_to_gdp: 1.2,
            },
            counties: Default::default(),
        };
        let events = vec![];
        let delta = bridge.tick(&world_state, &events).unwrap();
        assert_eq!(delta.gdp_growth_delta, 0.0);
    }

    #[test]
    fn bridge_processes_sector_shock() {
        let bridge = SimBridge::new().expect("Failed to init Python");
        let world_state = WorldStateSnapshot {
            week: 1,
            macro_state: MacroSnapshot {
                gdp_growth: 0.02,
                inflation: 0.03,
                unemployment: 0.04,
                fed_funds_rate: 0.05,
                consumer_confidence: 100.0,
                debt_to_gdp: 1.2,
            },
            counties: Default::default(),
        };
        let events = vec![SimEvent::SectorShock {
            sector: Sector::Energy,
            region: Some("Ohio".into()),
            severity: 0.7,
        }];
        let delta = bridge.tick(&world_state, &events).unwrap();
        assert!(!delta.sector_deltas.is_empty());
        assert!(!delta.narrative_seeds.is_empty());
    }
}
