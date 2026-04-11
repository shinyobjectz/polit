use std::path::PathBuf;

/// All game paths resolved from a single home directory (~/.polit)
pub struct GamePaths {
    pub root: PathBuf,
    pub saves: PathBuf,
    pub config: PathBuf,
    pub log: PathBuf,
    pub models: PathBuf,
    pub mods: PathBuf,
    pub meta: PathBuf,
}

impl GamePaths {
    /// Resolve all paths from ~/.polit, creating dirs as needed
    pub fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let root = if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".polit")
        } else {
            PathBuf::from(".polit")
        };

        let paths = Self {
            saves: root.join("saves"),
            config: root.join("config"),
            log: root.join("polit.log"),
            models: root.join("models"),
            mods: root.join("mods"),
            meta: root.join("meta"),
            root,
        };

        // Create all directories
        std::fs::create_dir_all(&paths.root)?;
        std::fs::create_dir_all(&paths.saves)?;
        std::fs::create_dir_all(&paths.config)?;
        std::fs::create_dir_all(&paths.models)?;
        std::fs::create_dir_all(&paths.mods)?;
        std::fs::create_dir_all(&paths.meta)?;

        // Seed default config files if they don't exist
        paths.seed_defaults()?;

        Ok(paths)
    }

    fn seed_defaults(&self) -> Result<(), Box<dyn std::error::Error>> {
        let balance_path = self.config.join("balance.toml");
        if !balance_path.exists() {
            std::fs::write(
                &balance_path,
                include_str!("../../game/config/balance.toml"),
            )?;
        }

        let difficulty_path = self.config.join("difficulty.toml");
        if !difficulty_path.exists() {
            std::fs::write(
                &difficulty_path,
                include_str!("../../game/config/difficulty.toml"),
            )?;
        }

        let theme_path = self.config.join("theme.toml");
        if !theme_path.exists() {
            std::fs::write(&theme_path, include_str!("../../game/config/theme.toml"))?;
        }

        Ok(())
    }
}
