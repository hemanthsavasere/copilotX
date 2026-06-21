use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub model: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default)]
    pub opencode_go_api_key: String,
    pub profile: String,
}

impl Config {
    pub fn load_from_path(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path))?;
        let config: Config = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config.json")?;
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let config_path = std::env::var("COPILOTX_CONFIG").unwrap_or_else(|_| {
            let mut p = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            p.push("copilotx");
            p.push("config.json");
            p.to_string_lossy().to_string()
        });
        Self::load_from_path(&config_path)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !matches!(
            self.model.as_str(),
            "gpt-4o" | "claude" | "claude-sonnet" | "kimi-k2.6"
        ) {
            errors.push(format!(
                "Unknown model: {}. Supported: gpt-4o, claude, claude-sonnet, kimi-k2.6",
                self.model
            ));
        }

        if self.model == "gpt-4o" && self.openai_api_key.is_empty() {
            errors.push("openaiApiKey is required when model is gpt-4o".to_string());
        }

        if matches!(self.model.as_str(), "claude" | "claude-sonnet")
            && self.anthropic_api_key.is_empty()
        {
            errors.push(
                "anthropicApiKey is required when model is claude/claude-sonnet".to_string(),
            );
        }

        if self.model == "kimi-k2.6" && self.opencode_go_api_key.is_empty() {
            errors.push("opencodeGoApiKey is required when model is kimi-k2.6".to_string());
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_valid_config_json() -> String {
        r#"{
            "model": "gpt-4o",
            "openaiApiKey": "sk-test",
            "anthropicApiKey": "",
            "profile": "interview"
        }"#
        .to_string()
    }

    #[test]
    fn test_load_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", make_valid_config_json()).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.model, "gpt-4o");
    }

    #[test]
    fn test_validate_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", make_valid_config_json()).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.validate().len(), 0);
    }

    #[test]
    fn test_validate_missing_api_key() {
        let json = r#"{
            "model": "gpt-4o",
            "openaiApiKey": "",
            "anthropicApiKey": "",
            "profile": "interview"
        }"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("openaiApiKey")));
    }

    #[test]
    fn test_validate_unknown_model() {
        let json = r#"{
            "model": "gpt-3",
            "openaiApiKey": "sk-test",
            "anthropicApiKey": "",
            "profile": "interview"
        }"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("Unknown model")));
    }

    #[test]
    fn test_load_missing_file() {
        let result = Config::load_from_path("/nonexistent/path/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_kimi_config() {
        let json = r#"{
            "model": "kimi-k2.6",
            "openaiApiKey": "",
            "anthropicApiKey": "",
            "opencodeGoApiKey": "sk-zen-test",
            "profile": "interview"
        }"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.validate().len(), 0);
    }

    #[test]
    fn test_validate_missing_opencode_go_api_key() {
        let json = r#"{
            "model": "kimi-k2.6",
            "openaiApiKey": "",
            "anthropicApiKey": "",
            "profile": "interview"
        }"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", json).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("opencodeGoApiKey")));
    }
}
