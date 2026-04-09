// Rhai scripting module — mod system (Phase 11)
// Stub for now

pub struct ScriptEngine {
    engine: rhai::Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = rhai::Engine::new();

        // Sandbox: disable dangerous operations
        engine.set_max_operations(100_000);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(1_000);
        engine.set_max_map_size(500);

        Self { engine }
    }

    pub fn eval(&self, script: &str) -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        self.engine.eval::<rhai::Dynamic>(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_script() {
        let engine = ScriptEngine::new();
        let result = engine.eval("40 + 2").unwrap();
        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[test]
    fn test_sandbox_operations_limit() {
        let engine = ScriptEngine::new();
        // Infinite loop should be stopped by operations limit
        let result = engine.eval("loop { }");
        assert!(result.is_err());
    }
}
