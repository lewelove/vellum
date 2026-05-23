pub mod assets;
pub mod playback;
pub mod system;
pub mod websocket;

use crate::server::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(websocket::ws_handler))
        .route("/api/state", post(system::update_state))
        .route("/api/internal/notify_force_update", post(system::notify_force_update))
        .route("/api/internal/reset", post(system::trigger_full_reset))
        .route("/api/internal/reload", post(system::trigger_reload))
        .route("/api/internal/batch_reload", post(system::trigger_batch_reload))
        .route("/api/internal/query", post(system::run_query))
        .route("/api/covers/{algo}/{size}/{hash}", get(assets::get_resized_cover))
        .route("/api/album/{*id}", get(assets::get_album_metadata))
        .route("/api/assets/cover/{*id}", get(assets::get_album_cover))
        .route("/api/assets/lyrics/{id}/{*path}", get(assets::get_lyrics))
        .route("/api/theme/shader", get(assets::get_custom_shader))
        .route("/api/theme/css", get(assets::get_custom_css))
        .route("/api/play/{*id}", post(playback::play_album))
        .route("/api/play-disc/{*id}", post(playback::play_disc))
        .route("/api/queue/{*id}", post(playback::queue_album))
        .route("/api/jump/{index}", post(playback::jump_to_index))
        .route("/api/next", post(playback::next_track))
        .route("/api/prev", post(playback::prev_track))
        .route("/api/stop", post(playback::stop_playback))
        .route("/api/clear", post(playback::clear_queue))
        .route("/api/toggle-pause", post(playback::toggle_pause))
        .route("/api/open/{*id}", post(system::open_album_folder))
        .route("/api/open-lock/{*id}", post(system::open_lock_file))
        .route("/api/open-manifest/{*id}", post(system::open_manifest_file))
        .route("/api/update-album/{*id}", post(system::force_update_album))
        .with_state(state)
}
