use serde::{Deserialize, Serialize};
use std::path::Path;

/// Core configuration for the DBeaver proxy.
///
/// Only the three essential fields are persisted in the config file:
/// - `base_url`: Backend provider URL
/// - `api_key`: Authentication key for the backend
/// - `model`: Default model name
///
/// Additional settings (port, timeout, logging, etc.) are configured
/// via environment variables or use sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Backend base URL (e.g., "https://api.openai.com/v1")
    pub base_url: String,

    /// API key for the backend provider
    pub api_key: String,

    /// Default model to advertise and use (e.g., "gpt-4o")
    pub model: String,
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// Returns `Ok(Config)` if the file exists and is valid TOML.
    /// Returns `Err` with a message if the file doesn't exist or is malformed.
    pub fn load(path: &str) -> Result<Self, String> {
        let config_path = Path::new(path);

        if !config_path.exists() {
            return Err(format!("Config file not found: {}", path));
        }

        let contents = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        let mut config: Config =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config file: {}", e))?;

        // Environment variable overrides (take precedence over config file)
        if let Ok(val) = std::env::var("DBEAVER_PROXY_BASE_URL") {
            config.base_url = val;
        } else if let Ok(val) = std::env::var("BASE_URL") {
            config.base_url = val;
        }

        if let Ok(val) = std::env::var("DBEAVER_PROXY_API_KEY") {
            config.api_key = val;
        } else if let Ok(val) = std::env::var("API_KEY") {
            config.api_key = val;
        }

        if let Ok(val) = std::env::var("DBEAVER_PROXY_MODEL") {
            config.model = val;
        } else if let Ok(val) = std::env::var("MODEL") {
            config.model = val;
        }

        // Validation
        if config.api_key.is_empty() {
            return Err(
                "API key is required. Set DBEAVER_PROXY_API_KEY or run `dbeaver-proxy init`."
                    .to_string(),
            );
        }

        Ok(config)
    }

    /// Write configuration to a TOML file.
    ///
    /// Creates parent directories if they don't exist.
    /// Returns the path where the file was written on success.
    pub fn write(&self, path: &str) -> Result<String, String> {
        let config_path = Path::new(path);

        // Create parent directories if needed
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(config_path, contents)
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(config_path.to_string_lossy().to_string())
    }

    /// Validate configuration values.
    ///
    /// Returns a list of warnings for non-critical issues
    /// (e.g., unreachable base URL, unusual port).
    #[allow(dead_code)]
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if !self.base_url.starts_with("http") {
            warnings.push(format!(
                "Base URL does not start with http/https: {}",
                self.base_url
            ));
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn setup_test_config() -> (tempfile::NamedTempFile, Config) {
        let config = Config {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-key".to_string(),
            model: "gpt-4o".to_string(),
        };

        let mut file = tempfile::NamedTempFile::new().unwrap();
        let content = toml::to_string_pretty(&config).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        (file, config)
    }

    #[test]
    fn test_config_load_valid() {
        let (file, expected) = setup_test_config();
        let path = file.path().to_string_lossy().to_string();
        let config = Config::load(&path).unwrap();

        assert_eq!(config.base_url, expected.base_url);
        assert_eq!(config.api_key, expected.api_key);
        assert_eq!(config.model, expected.model);
    }

    #[test]
    fn test_config_load_missing_file() {
        let result = Config::load("/nonexistent/path.toml");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_config_load_invalid_toml() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        file.write_all(b"invalid toml {{{").unwrap();

        let path = file.path().to_string_lossy().to_string();
        let result = Config::load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation() {
        let config = Config {
            base_url: "not-a-url".to_string(),
            api_key: "sk-test".to_string(),
            model: "gpt-4o".to_string(),
        };

        let warnings = config.validate();
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("not-a-url"));
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config {
            base_url: "https://api.mistral.ai/v1".to_string(),
            api_key: "sk-mistral-key".to_string(),
            model: "mistral-large-latest".to_string(),
        };

        let dir = std::env::temp_dir();
        let path = dir.join("test-dbeaver-proxy.toml");
        let path_str = path.to_string_lossy().to_string();

        // Write
        config.write(&path_str).unwrap();

        // Read back
        let loaded = Config::load(&path_str).unwrap();

        assert_eq!(loaded.base_url, config.base_url);
        assert_eq!(loaded.api_key, config.api_key);
        assert_eq!(loaded.model, config.model);

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}
