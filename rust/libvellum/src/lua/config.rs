use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub manifest: ManifestConfig,
    #[serde(default)]
    pub compiler: CompilerConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct InterfaceConfig {
    #[serde(default)]
    pub enable: bool,
    pub directory: Option<String>,
    pub run: Option<String>,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub assets: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ActionConfig {
    pub run: Option<String>,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    #[serde(default)]
    pub library: String,
    pub environment: Option<String>,
    #[serde(default = "default_cache")]
    pub cache: String,
    #[serde(default = "default_state")]
    pub state: String,
}

fn default_cache() -> String {
    "~/.cache/vellum".to_string()
}
fn default_state() -> String {
    "~/.local/share/vellum".to_string()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            library: String::new(),
            environment: None,
            cache: default_cache(),
            state: default_state(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ManifestConfig {
    pub audio_files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CompilerConfig {
    pub date_added: Option<Vec<String>>,
    pub manifests: Option<Vec<String>>,
    pub scan_depth: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CoversConfig {
    pub interpolation: Option<String>,
    pub size: u32,
}
