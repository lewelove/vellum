use crate::server::mpd::MpdEngine;
use crate::server::query::QueryEngine;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use indexmap::IndexMap;
use libvellum::lua::config::{CoversConfig, InterfaceConfig, ActionConfig};

pub struct AppState {
    pub query: Arc<RwLock<QueryEngine>>,
    pub ui_state: RwLock<serde_json::Value>,
    pub tx: broadcast::Sender<String>,
    pub config: RwLock<AppConfig>,
    pub mpd_engine: MpdEngine,
}

#[derive(Clone)]
pub struct AppConfig {
    pub library_root: PathBuf,
    pub cache_root: PathBuf,
    pub state_root: PathBuf,
    pub resolved_logic_path: Option<PathBuf>,
    pub resolved_shelf_files: Vec<PathBuf>,
    pub resolved_dependencies: Vec<PathBuf>,
    pub covers: IndexMap<String, CoversConfig>,
    pub interfaces: std::collections::HashMap<String, InterfaceConfig>,
    pub actions: std::collections::HashMap<String, ActionConfig>,
    pub config_dir: PathBuf,
    pub app: libvellum::lua::config::AppConfig,
}
