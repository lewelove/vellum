use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub storage: StorageConfig,
    pub theme: Option<ThemeConfig>,
    pub manifest: Option<ManifestConfig>,
    pub compiler: Option<CompilerConfig>,
    pub actions: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub library_root: String,
    pub library_export: Option<String>,
    #[serde(default = "default_cache")]
    pub cache: String,
    #[serde(default = "default_state")]
    pub state: String,
    pub environment: Option<String>,
}

fn default_cache() -> String {
    "~/.cache/vellum".to_string()
}
fn default_state() -> String {
    "~/.local/share/vellum".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ThemeConfig {
    pub shader: Option<ShaderConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShaderConfig {
    pub path: Option<String>,
    pub speed: Option<f32>,
    pub zoom: Option<f32>,
    pub blur: Option<f32>,
    pub grain: Option<f32>,
    pub equalize: Option<f32>,
    pub order: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManifestConfig {
    pub supported_extensions: Option<Vec<String>>,
    pub keys: Option<IndexMap<String, ManifestKeyConfig>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManifestKeyConfig {
    pub level: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[serde(default)]
    pub newline: bool,
    pub manifests: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CompilerKeys {
    #[serde(default)]
    pub album: HashMap<String, KeyConfig>,
    #[serde(default)]
    pub tracks: HashMap<String, KeyConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompilerConfig {
    pub scan_depth: Option<usize>,
    pub keys: Option<CompilerKeys>,
    pub date_added: Option<Vec<String>>,
    pub file_subset_match: Option<Vec<String>>,
    pub manifests: Option<Vec<String>>,
    #[serde(default)]
    pub covers: IndexMap<String, CoversConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CoversConfig {
    pub interpolation: Option<String>,
    pub size: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyConfig {
    #[serde(rename = "type")]
    pub type_: crate::types::VellumDataType,
    #[serde(default)]
    pub args: String,
}

impl AppConfig {
    pub fn load() -> Result<(Self, Value, PathBuf)> {
        let config_path = Self::resolve_config_path()
            .context("Could not locate config.toml in home directory or project hierarchy")?;

        let mut visited = std::collections::HashSet::new();
        let raw_value = Self::load_recursive(&config_path, &mut visited)?;

        let mut config: Self = Value::try_into(raw_value.clone())?;

        if let Some(ref mut compiler) = config.compiler {
            if compiler.covers.is_empty() {
                compiler.covers.insert("master".to_string(), CoversConfig { interpolation: Some("mitchell".to_string()), size: 1080 });
                compiler.covers.insert("thumbnail".to_string(), CoversConfig { interpolation: Some("lanczos".to_string()), size: 190 });
            } else if !compiler.covers.contains_key("master") {
                compiler.covers.insert("master".to_string(), CoversConfig { interpolation: Some("mitchell".to_string()), size: 1080 });
            }
        } else {
            let mut covers = IndexMap::new();
            covers.insert("master".to_string(), CoversConfig { interpolation: Some("mitchell".to_string()), size: 1080 });
            covers.insert("thumbnail".to_string(), CoversConfig { interpolation: Some("lanczos".to_string()), size: 190 });
            config.compiler = Some(CompilerConfig {
                scan_depth: None,
                keys: None,
                date_added: None,
                file_subset_match: None,
                manifests: None,
                covers,
            });
        }

        Ok((config, raw_value, config_path))
    }

    fn resolve_config_path() -> Option<PathBuf> {
        if let Some(home_config) = dirs::home_dir().map(|h| h.join(".config/vellum/config.toml"))
            && home_config.exists() {
                return Some(home_config);
            }

        if let Ok(env_path) = std::env::var("VELLUM_CONFIG_PATH") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return Some(p);
            }
        }

        let mut curr = std::env::current_dir().ok()?;
        loop {
            let local_nested = curr.join("config/config.toml");
            if local_nested.exists() {
                return Some(local_nested);
            }

            let local_root = curr.join("config.toml");
            if local_root.exists() {
                return Some(local_root);
            }

            if let Some(parent) = curr.parent() {
                curr = parent.to_path_buf();
            } else {
                break;
            }
        }

        None
    }

    fn load_recursive(
        path: &Path,
        visited: &mut std::collections::HashSet<PathBuf>,
    ) -> Result<Value> {
        let canon_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !visited.insert(canon_path) {
            return Err(anyhow::anyhow!(
                "Circular import detected: {}",
                path.display()
            ));
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let mut current_value: Value = toml::from_str(&content)?;

        if let Some(imports) = current_value.get("import") {
            let import_paths = match imports {
                Value::String(s) => vec![s.clone()],
                Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect(),
                _ => vec![],
            };

            let mut merged_base = Value::Table(toml::map::Map::new());
            let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

            for rel_path in import_paths {
                let abs_path = base_dir.join(rel_path);
                let imported_value = Self::load_recursive(&abs_path, visited)?;
                merged_base = Self::deep_merge(merged_base, imported_value);
            }

            current_value = Self::deep_merge(merged_base, current_value);
        }

        Ok(current_value)
    }

    fn deep_merge(base: Value, overlay: Value) -> Value {
        match (base, overlay) {
            (Value::Table(mut base_map), Value::Table(overlay_map)) => {
                for (k, v) in overlay_map {
                    if k == "import" {
                        continue;
                    }
                    let base_v = base_map.remove(&k);
                    let merged_v = match base_v {
                        Some(bv) => Self::deep_merge(bv, v),
                        None => v,
                    };
                    base_map.insert(k, merged_v);
                }
                Value::Table(base_map)
            }
            (_, overlay) => overlay,
        }
    }
}
