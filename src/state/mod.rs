//! File-based game state. All game data lives as YAML/markdown files
//! in ~/.polit/saves/<save_name>/. Save = copy directory. Load = read directory.

pub mod gamestate_fs;

pub use gamestate_fs::GameStateFs;
