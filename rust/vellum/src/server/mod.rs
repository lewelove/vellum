mod api;
mod library;
mod mpd;
mod query;
mod state;
mod inotify;

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};
use tower_http::cors::{Any, CorsLayer};

use self::state::{AppConfig as ServerConfig, AppState};
use libvellum::utils::expand_path;

fn create_directories(cache_root: &Path, state_root: &Path) {
    std::fs::create_dir_all(cache_root).ok();
    std::fs::create_dir_all(cache_root.join("covers").join("master")).ok();
    std::fs::create_dir_all(cache_root.join("covers").join("static")).ok();
    std::fs::create_dir_all(cache_root.join("covers").join("dynamic")).ok();
    std::fs::create_dir_all(cache_root.join("cover_data")).ok();
    std::fs::create_dir_all(state_root).ok();
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
        "homeSubView": "library",
        "activeLibrary": "library",
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
    let config = libvellum::lua::ResolvedConfig::load().context("Failed to load application configuration")?;
    let config_dir = config.path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    let lib_root_str = &config.app.storage.library;

    let library_root = expand_path(lib_root_str)
        .canonicalize()
        .context("Invalid library_root path")?;

    let cache_root = expand_path(&config.app.storage.cache);
    let state_root = expand_path(&config.app.storage.state);

    create_directories(&cache_root, &state_root);

    let covers = config.covers.clone();
    let interfaces = config.interfaces.clone();
    let resolved_dependencies = config.dependencies.clone();
    
    let logic_path = config_dir.join("logic.toml");
    let resolved_logic_path = logic_path.canonicalize().ok();

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
        resolved_logic_path,
        resolved_shelf_files,
        resolved_dependencies,
        covers,
        interfaces,
        config_dir: config_dir.clone(),
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

    inotify::start(Arc::clone(&app_state));

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
