use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub hotkey: String,
    pub model: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub anthropic_api_key: String,
    pub profile: String,
    pub overlay_opacity: f64,
    pub overlay_width: u32,
    pub overlay_position: String,
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

        if !matches!(self.model.as_str(), "gpt-4o" | "claude" | "claude-sonnet") {
            errors.push(format!(
                "Unknown model: {}. Supported: gpt-4o, claude, claude-sonnet",
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

        if self.hotkey.is_empty() {
            errors.push("hotkey is required".to_string());
        }

        if self.overlay_opacity < 0.1 || self.overlay_opacity > 1.0 {
            errors.push("overlayOpacity must be between 0.1 and 1.0".to_string());
        }

        if self.overlay_width < 200 || self.overlay_width > 800 {
            errors.push("overlayWidth must be between 200 and 800".to_string());
        }

        let valid_positions = ["left", "right", "top", "bottom"];
        if !valid_positions.contains(&self.overlay_position.as_str()) {
            errors.push(format!(
                "Unknown overlayPosition: {}. Supported: {}",
                self.overlay_position,
                valid_positions.join(", ")
            ));
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
            "hotkey": "CommandOrControl+Shift+Space",
            "model": "gpt-4o",
            "openaiApiKey": "sk-test",
            "anthropicApiKey": "",
            "profile": "interview",
            "overlayOpacity": 0.85,
            "overlayWidth": 320,
            "overlayPosition": "right"
        }"#
        .to_string()
    }

    #[test]
    fn test_load_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", make_valid_config_json()).unwrap();
        let config = Config::load_from_path(f.path().to_str().unwrap()).unwrap();
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.hotkey, "CommandOrControl+Shift+Space");
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
            "hotkey": "CommandOrControl+Shift+Space",
            "model": "gpt-4o",
            "openaiApiKey": "",
            "anthropicApiKey": "",
            "profile": "interview",
            "overlayOpacity": 0.85,
            "overlayWidth": 320,
            "overlayPosition": "right"
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
            "hotkey": "CommandOrControl+Shift+Space",
            "model": "gpt-3",
            "openaiApiKey": "sk-test",
            "anthropicApiKey": "",
            "profile": "interview",
            "overlayOpacity": 0.85,
            "overlayWidth": 320,
            "overlayPosition": "right"
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
}
