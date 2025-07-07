use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// User's display name for chat messages
    #[serde(default = "default_username")]
    pub username: String,
    
    /// Disable AI/LLM functionality
    #[serde(default)]
    pub disable_ai: bool,
    
    /// Default language for translations
    #[serde(default = "default_target_language")]
    pub target_language: String,
}

fn default_username() -> String {
    "Anonymous".to_string()
}

fn default_target_language() -> String {
    "Spanish".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            username: default_username(),
            disable_ai: false,
            target_language: default_target_language(),
        }
    }
}

impl Config {
    /// Get the default config file path: ~/.config/puf/config.toml
    pub fn default_config_path() -> Result<PathBuf> {
        let home_dir = std::env::home_dir()
            .context("Could not determine home directory")?;
        
        Ok(home_dir.join(".config").join("puf").join("config.toml"))
    }
    
    /// Load config from a file path, creating default config if file doesn't exist
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            
            Ok(config)
        } else {
            // Create default config and save it
            let config = Config::default();
            config.save_to_path(path)?;
            Ok(config)
        }
    }
    
    /// Load config from default location or provided override
    pub fn load(config_path_override: Option<PathBuf>) -> Result<Self> {
        let config_path = config_path_override
            .unwrap_or_else(|| Self::default_config_path().unwrap_or_else(|_| {
                // Fallback to current directory if home detection fails
                PathBuf::from("config.yaml")
            }));
        
        Self::load_from_path(&config_path)
    }
    
    /// Save config to a file path
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        
        let toml_content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;
        
        fs::write(path, toml_content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        tracing::info!("Saved config to: {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.username, "Anonymous");
        assert!(!config.disable_ai);
        assert_eq!(config.target_language, "Spanish");
    }
    
    #[test]
    fn test_config_serialization() {
        let config = Config {
            username: "TestUser".to_string(),
            disable_ai: true,
            target_language: "French".to_string(),
        };
        
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.username, deserialized.username);
        assert_eq!(config.disable_ai, deserialized.disable_ai);
        assert_eq!(config.target_language, deserialized.target_language);
    }
    
    #[test]
    fn test_config_load_save() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("test_config.toml");
        
        let original_config = Config {
            username: "TestUser".to_string(),
            disable_ai: true,
            target_language: "German".to_string(),
        };
        
        // Save config
        original_config.save_to_path(&config_path)?;
        
        // Load config
        let loaded_config = Config::load_from_path(&config_path)?;
        
        assert_eq!(original_config.username, loaded_config.username);
        assert_eq!(original_config.disable_ai, loaded_config.disable_ai);
        assert_eq!(original_config.target_language, loaded_config.target_language);
        
        Ok(())
    }
    
    #[test]
    fn test_config_load_nonexistent_creates_default() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("nonexistent_config.toml");
        
        assert!(!config_path.exists());
        
        let config = Config::load_from_path(&config_path)?;
        
        // Should have created the file with default values
        assert!(config_path.exists());
        assert_eq!(config.username, "Anonymous");
        assert!(!config.disable_ai);
        assert_eq!(config.target_language, "Spanish");
        
        Ok(())
    }
}