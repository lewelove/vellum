pub mod api;
pub mod library;
pub mod mpd;
pub mod query;
pub mod state;
pub mod watchdog;

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};
use tower_http::cors::{Any, CorsLayer};

use self::state::{AppConfig as ServerConfig, AppState};
use libvellum::config::AppConfig;
use libvellum::utils::expand_path;

fn create_directories(cache_root: &Path, state_root: &Path) {
    std::fs::create_dir_all(cache_root).ok();
    std::fs::create_dir_all(cache_root.join("covers").join("master")).ok();
    std::fs::create_dir_all(cache_root.join("covers").join("static")).ok();
    std::fs::create_dir_all(cache_root.join("covers").join("dynamic")).ok();
    std::fs::create_dir_all(cache_root.join("cover_data")).ok();
    std::fs::create_dir_all(state_root).ok();
}

fn resolve_compiler_paths(
    config_dir: &Path,
    shader_cfg: Option<&libvellum::config::ShaderConfig>,
) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    let resolved_shader_path = if let Some(s) = shader_cfg
        && let Some(p) = &s.path {
            let expanded = expand_path(p);
            let absolute = if expanded.is_absolute() {
                expanded
            } else {
                config_dir.join(expanded)
            };
            absolute.canonicalize().ok().or(Some(absolute))
        } else {
            None
        };

    let css_path = config_dir.join("vellum.css");
    let resolved_css_path = css_path.canonicalize().ok();

    let logic_path = config_dir.join("logic.toml");
    let resolved_logic_path = logic_path.canonicalize().ok();

    (resolved_shader_path, resolved_css_path, resolved_logic_path)
}

fn load_state(state_root: &Path) -> serde_json::Value {
    let state_file = state_root.join("state.json");
    if state_file.exists() {
        let data = std::fs::read_to_string(&state_file).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_else(|_| default_state())
    } else {
        default_state()
    }
}

fn default_state() -> serde_json::Value {
    serde_json::json!({
        "activeTab": "home",
        "activeCollection": "library",
        "sortKey": "default",
        "sortOrder": "default",
        "groupKey": "genre",
        "filter": {
            "key": null,
            "val": null
        },
        "queuePanels": {
            "lyrics": false,
            "tracks": true
        },
        "sidebarWidth": 280
    })
}

pub async fn run(port: u16) -> Result<()> {
    let (config, _, config_path): (AppConfig, toml::Value, PathBuf) = AppConfig::load().context("Failed to load application configuration")?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    let lib_root_str = &config.storage.library_root;

    let library_root = expand_path(lib_root_str)
        .canonicalize()
        .context("Invalid library_root path")?;

    let cache_root = expand_path(&config.storage.cache);
    let state_root = expand_path(&config.storage.state);

    create_directories(&cache_root, &state_root);

    let shader_cfg = config.theme.as_ref().and_then(|t| t.shader.clone());
    let covers = config.compiler.as_ref().map(|c| c.covers.clone()).unwrap_or_default();
    
    let (resolved_shader_path, resolved_css_path, resolved_logic_path) = resolve_compiler_paths(&config_dir, shader_cfg.as_ref());

    let mut query_engine = query::QueryEngine::new()?;
    
    let mut resolved_shelf_files = Vec::new();
    for shelf in query_engine.manifest.shelves.values() {
        if let Some(file) = &shelf.file {
            let expanded = expand_path(file);
            resolved_shelf_files.push(expanded.canonicalize().unwrap_or(expanded));
        }
    }

    let server_config = ServerConfig {
        library_root: library_root.clone(),
        cache_root,
        state_root: state_root.clone(),
        shader: shader_cfg,
        resolved_shader_path,
        resolved_css_path,
        resolved_logic_path,
        resolved_shelf_files,
        covers,
    };

    let ui_state_val = load_state(&state_root);

    let lib_scanner = library::scanner::Library::new(library_root.clone());
    lib_scanner.scan(&mut query_engine);
    
    let query_arc = Arc::new(Mutex::new(query_engine));
    let (tx, _) = broadcast::channel(100);

    let mpd_engine = mpd::start_actor(
        tx.clone(),
        Arc::clone(&query_arc),
        Arc::new(server_config.clone()),
    );

    let app_state = Arc::new(AppState {
        query: Arc::clone(&query_arc),
        ui_state: RwLock::new(ui_state_val),
        tx,
        config: RwLock::new(server_config),
        mpd_engine,
    });

    watchdog::start(&config_path, Arc::clone(&app_state));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = api::router(Arc::clone(&app_state)).layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    log::info!("Vellum Server listening on http://{addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
