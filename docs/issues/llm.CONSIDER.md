# llm.CONSIDER.md

Here lies critique by LLMs on anti-patterns in current codebase selected by me from conversations about it.

---

#### Faux "Entropy" Calculation via File Compression
* **`rust/libvellum/src/images/cover_entropy.rs`**:
The function `calculate_entropy` is implemented by converting a grayscale image to a PNG in-memory and measuring the resulting compressed buffer length. 
Using a lossless compression algorithm's byte length as a proxy for visual entropy incurs massive CPU, memory allocation, and compression overhead. Mathematical Shannon Entropy ($-\sum p_i \log_2 p_i$) can be computed linearly $O(N)$ with a simple intensity histogram, which is far cheaper and mathematically correct.

#### Dummy Dependency Tracking in Effects
* **`web-app/src/modules/HomeView/AlbumGrid/Album.svelte`**:
An `$effect` declares a dummy object `_` consisting of reactive properties solely to register them as tracking dependencies for Svelte's reactive system, because the actual rendering function (`renderText()`) operates imperatively on a 2D Canvas context. 
This is a reactive anti-pattern. A cleaner alternative is to pass the tracked state variables explicitly as parameters to `renderText(title, artist, coverSize, ...)` to preserve automatic dependency tracking naturally.

#### Redundant Event Polling Loops
* **`web-app/src/modules/QueueView/Control.svelte`** & **`web-app/src/modules/QueueView/ControlPanel.svelte`**
Both components duplicate the exact same ticking interval logic (`setInterval`) to calculate and update track playback times. 
This introduces a high probability of desynchronization between different sections of the UI and wastes CPU resources. This progressive tick should be managed centrally in a single store (such as `player.svelte.ts`) and exposed reactively to any consumer components.

#### Race Conditions in Uncontrolled Async Effects
* **`web-app/src/modules/QueueView/QueueView.svelte`**
An `$effect` maps over all items in `player.queue` and triggers `library.ensureFullAlbum(id)` asynchronously on each unique album ID. 
Spawning numerous asynchronous fetches inside an effect without cancellation tokens or tracking of in-flight promises can lead to classic race conditions, out-of-order state mutations, and API rate-limiting/spamming if the queue changes rapidly.

#### Blocking the Async Executor with Synchronous File I/O
Tokio’s multi-threaded scheduler relies on cooperative multitasking. Executing synchronous, blocking I/O calls directly on an async thread starves the executor, degrading response times for all other concurrent tasks.
* **`rust/vellum/src/server/api/assets.rs`**:
In the async route handler `get_resized_cover`, the helper `find_cached_cover` is called directly. It executes synchronous `.exists()` checks on multiple files on the main async task.
* **`rust/vellum/src/update/mod.rs`**:
The `start_notification_task` spawns an asynchronous Tokio task (`tokio::spawn`). Inside this loop, it calls `get_mtime_sum` for every updated album. `get_mtime_sum` runs a deeply nested synchronous `walkdir::WalkDir` traversal and queries `fs::metadata` synchronously on the async executor thread.
* **`rust/vellum/src/server/watchdog/mod.rs`**:
The async filesystem watchdog handler (`handle_logic_change`) performs synchronous path canonicalizations and calls `query.reload_manifest()`, which synchronously reads and parses TOML files from disk.

#### Holding Tokio Locks Across Heavy Blocking Operations
* **`rust/vellum/src/server/api/system.rs`**: In the `trigger_full_reset` handler, the code acquires a write lock on the global query engine via `let mut query = state.query.lock().await;` and then invokes `scanner.scan(&mut query);` while holding the lock. The `scan` operation performs a synchronous disk traversal to read all `metadata.lock.json` files in the library. While this is running, **every other endpoint or WebSocket query requiring access to `state.query` is blocked**, completely freezing database queries and playback commands during full rescans.

#### Arbitrary SQL Execution HTTP Route
* **`rust/vellum/src/server/api/system.rs`**:
The `/api/internal/query` endpoint accepts a raw `query` string parameter, expands its shorthand, and immediately executes it on the database via `query_ids`. 
This exposes the SQLite instance to arbitrary SQL execution. If the server is bound to a public interface or if local network actors have access, an attacker could manipulate, read, or overwrite arbitrary database entries.

#### Full Deep-Cloning of Catalogs Over WebSockets
* **`rust/vellum/src/server/api/websocket.rs`**
Upon client connection, the `init_payload` locks the query engine, deep-clones three large in-memory collection maps (`q.dict.clone()`, `q.track_lookup.clone()`, `q.manifest.clone()`), and serializes them into a massive JSON payload sent over the socket. 
As library sizes grow to thousands of albums and tracks, this operation will cause noticeable memory allocation spikes, hold mutexes longer than necessary, and consume extensive network bandwidth.
