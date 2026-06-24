use anyhow::Result;
use indexmap::IndexMap;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub use libvellum::sql::expand_shorthand;

fn deserialize_vec_or_string_opt<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum VecOrString {
        Vec(Vec<String>),
        String(String),
    }

    let opt = Option::<VecOrString>::deserialize(deserializer)?;
    Ok(opt.map(|v| match v {
        VecOrString::Vec(vec) => vec,
        VecOrString::String(s) => vec![s.trim().to_string()],
    }))
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct SqlDef {
    pub select: Option<String>,
    #[serde(rename = "where")]
    pub where_: Option<String>,
    pub order_by: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct FilterDef {
    pub label: String,
    #[serde(default)]
    pub sql: SqlDef,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LogicManifest {
    #[serde(default)]
    pub filters: IndexMap<String, FilterDef>,
    pub groupers: IndexMap<String, GrouperDef>,
    pub orders: IndexMap<String, OrderDef>,
    pub libraries: IndexMap<String, LibraryDef>,
    #[serde(default)]
    pub shelves: IndexMap<String, ShelfDef>,
    #[serde(skip_deserializing, default)]
    pub filters_order: Vec<String>,
    #[serde(skip_deserializing, default)]
    pub groupers_order: Vec<String>,
    #[serde(skip_deserializing, default)]
    pub orders_order: Vec<String>,
    #[serde(skip_deserializing, default)]
    pub libraries_order: Vec<String>,
    #[serde(skip_deserializing, default)]
    pub shelves_order: Vec<String>,
}

impl LogicManifest {
    pub fn normalize(&mut self) {
        self.filters_order = self.filters.keys().cloned().collect();
        self.groupers_order = self.groupers.keys().cloned().collect();
        self.orders_order = self.orders.keys().cloned().collect();
        self.libraries_order = self.libraries.keys().cloned().collect();
        self.shelves_order = self.shelves.keys().cloned().collect();

        for (_, g) in &mut self.groupers {
            let idx = g.index.unwrap_or(false);
            g.index = Some(idx);
            if g.count.is_none() {
                g.count = Some(!idx);
            }
        }

        let global_groupers: HashSet<String> = self.groupers.iter()
            .filter(|(_, g)| !g.strict)
            .map(|(id, _)| id.clone())
            .collect();

        let global_orders: HashSet<String> = self.orders.iter()
            .filter(|(_, s)| !s.strict)
            .map(|(id, _)| id.clone())
            .collect();

        for (_, library) in &mut self.libraries {
            let mut allowed_filter_ids = Vec::new();
            if let Some(filters) = &library.filters {
                for f in filters {
                    if self.filters.contains_key(f) {
                        allowed_filter_ids.push(f.clone());
                    }
                }
            }
            library.allowed_filters = allowed_filter_ids;

            let mut allowed_grouper_ids = HashSet::new();
            for g in &library.groupers {
                allowed_grouper_ids.insert(g.clone());
            }
            if !library.strict {
                for g in &global_groupers {
                    allowed_grouper_ids.insert(g.clone());
                }
            }

            let mut allowed_order_ids = HashSet::new();
            for s in &library.orders {
                allowed_order_ids.insert(s.clone());
            }
            if !library.strict {
                for s in &global_orders {
                    allowed_order_ids.insert(s.clone());
                }
            }

            library.allowed_groupers = self.groupers.keys()
                .filter(|k| allowed_grouper_ids.contains(*k))
                .cloned()
                .collect();

            library.allowed_orders = self.orders.keys()
                .filter(|k| allowed_order_ids.contains(*k))
                .cloned()
                .collect();
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GrouperDef {
    pub label: String,
    #[serde(default)]
    pub sql: SqlDef,
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub index: Option<bool>,
    #[serde(default)]
    pub count: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OrderDef {
    pub label: String,
    #[serde(default)]
    pub sql: SqlDef,
    #[serde(default)]
    pub strict: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LibraryDef {
    pub label: String,
    #[serde(default)]
    pub sql: Option<SqlDef>,
    #[serde(default, deserialize_with = "deserialize_vec_or_string_opt")]
    pub filters: Option<Vec<String>>,
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub groupers: Vec<String>,
    #[serde(default)]
    pub orders: Vec<String>,
    #[serde(skip_deserializing)]
    pub allowed_filters: Vec<String>,
    #[serde(skip_deserializing)]
    pub allowed_groupers: Vec<String>,
    #[serde(skip_deserializing)]
    pub allowed_orders: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ShelfDef {
    pub label: String,
    #[serde(default)]
    pub sql: Option<SqlDef>,
    pub file: Option<String>,
}

pub struct QueryEngine {
    conn: Connection,
    pub manifest: LogicManifest,
    libraries_cache: HashMap<String, HashSet<u32>>,
    filters_cache: HashMap<String, HashSet<u32>>,
    facets_cache: HashMap<String, HashMap<String, HashSet<u32>>>,
    orders_cache: HashMap<String, Vec<u32>>,
    shelves_cache: HashMap<String, Vec<u32>>,
    uid_to_id: HashMap<u32, String>,
    pub dict: HashMap<String, Value>,
    pub track_lookup: HashMap<String, Value>,
    pub path_lookup: HashMap<String, String>,
}

const DEFAULT_LOGIC: &str = include_str!("logic.toml");

impl QueryEngine {
    pub fn new() -> Result<Self> {
        let logic_path = crate::expand_path("~/.config/vellum/logic.toml");
        if !logic_path.exists() {
            std::fs::write(&logic_path, DEFAULT_LOGIC)?;
        }

        let logic_content = std::fs::read_to_string(&logic_path)?;
        let mut manifest: LogicManifest = toml::from_str(&logic_content)?;
        manifest.normalize();

        let conn = Connection::open_in_memory()?;
        conn.execute(
            "CREATE TABLE albums (
                uid INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT UNIQUE,
                metadata TEXT
            )",[],
        )?;

        Ok(Self {
            conn,
            manifest,
            libraries_cache: HashMap::new(),
            filters_cache: HashMap::new(),
            facets_cache: HashMap::new(),
            orders_cache: HashMap::new(),
            shelves_cache: HashMap::new(),
            uid_to_id: HashMap::new(),
            dict: HashMap::new(),
            track_lookup: HashMap::new(),
            path_lookup: HashMap::new(),
        })
    }

    pub fn reload_manifest(&mut self, path: &Path) -> Result<()> {
        let logic_content = std::fs::read_to_string(path)?;
        let mut manifest: LogicManifest = toml::from_str(&logic_content)?;
        manifest.normalize();
        self.manifest = manifest;
        self.build_cache()?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM albums",[])?;
        self.libraries_cache.clear();
        self.filters_cache.clear();
        self.facets_cache.clear();
        self.orders_cache.clear();
        self.shelves_cache.clear();
        self.uid_to_id.clear();
        self.dict.clear();
        self.track_lookup.clear();
        self.path_lookup.clear();
        Ok(())
    }

    pub fn remove_album(&mut self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM albums WHERE id = ?1", [&id])?;
        self.dict.remove(id);
        self.uid_to_id.retain(|_, v| v != id);
        self.path_lookup.retain(|_, v| v != id);
        self.track_lookup.retain(|_, v| v.get("albumId").and_then(|a| a.as_str()) != Some(id));
        Ok(())
    }

    pub fn ingest(&mut self, id: &str, metadata_json: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO albums (id, metadata) VALUES (?1, ?2)",[id, metadata_json],
        )?;
        let uid = u32::try_from(self.conn.last_insert_rowid()).unwrap_or(0);
        self.uid_to_id.insert(uid, id.to_string());

        if let Ok(parsed) = serde_json::from_str::<Value>(metadata_json)
            && let Some(album) = parsed.get("album")
                && let Some(info) = album.get("info") {
                    if let Some(tracks) = parsed.get("tracks").and_then(Value::as_array) {
                        for track in tracks {
                            if let Some(tinfo) = track.get("info") {
                                let rel = track.get("file").and_then(|f| f.get("path")).and_then(Value::as_str).unwrap_or("");
                                let tp_path = Path::new(id).join(rel);
                                let tp = tp_path.to_string_lossy().to_string();

                                let track_no = track.get("tracknumber").cloned().unwrap_or_else(|| json!(0));
                                let disc_no = track.get("discnumber").cloned().unwrap_or_else(|| json!(1));
                                let title = track.get("title").cloned().unwrap_or_else(|| json!("Unknown"));
                                let artist = track.get("artist").cloned().unwrap_or_else(|| json!("Unknown"));
                                let duration = tinfo.get("duration_formatted").cloned().unwrap_or_else(|| json!("0:00"));
                                let duration_ms = tinfo.get("duration_milliseconds").and_then(Value::as_u64).unwrap_or(0);

                                let track_light = json!({
                                    "path": tp,
                                    "trackNo": track_no,
                                    "discNo": disc_no,
                                    "title": title,
                                    "artist": artist,
                                    "duration": duration,
                                    "durationMs": duration_ms,
                                    "albumId": id
                                });
                                self.track_lookup.insert(tp.clone(), track_light);

                                let full_rel_path = Path::new(id).join(rel);
                                let normalized = full_rel_path.to_string_lossy().trim_start_matches('/').to_string();
                                self.path_lookup.insert(normalized, id.to_string());
                            }
                        }
                    }

                    let entry = json!({
                        "id": id,
                        "album": album.get("album"),
                        "albumartist": album.get("albumartist"),
                        "date": album.get("date"),
                        "genre": album.get("genre"),
                        "cover_path": album.get("covers").and_then(|c| c.get("main")).and_then(|m| m.get("file")).and_then(|f| f.get("path")),
                        "cover_hash": album.get("covers").and_then(|c| c.get("main")).and_then(|m| m.get("file")).and_then(|f| f.get("hash")).and_then(|h| h.get("address")),
                        "duration_formatted": info.get("duration_formatted"),
                        "total_discs": album.get("total_discs"),
                        "total_tracks": album.get("total_tracks"),
                        "unix_added": info.get("date_added"),
                        "keys": album.get("keys")
                    });
                    self.dict.insert(id.to_string(), entry);
                }

        Ok(())
    }

    pub fn query_ids(&self, sql: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(sql)?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(ids)
    }

    pub fn build_cache(&mut self) -> Result<()> {
        self.filters_cache.clear();
        for (key, filter) in &self.manifest.filters {
            let where_str = filter.sql.where_.as_deref().unwrap_or("1=1");
            let expanded = expand_shorthand(where_str);
            let sql = format!("SELECT uid FROM albums WHERE {expanded}");
            if let Ok(mut stmt) = self.conn.prepare(&sql) {
                let uids: HashSet<u32> = stmt.query_map([], |row| row.get(0))?.filter_map(Result::ok).collect();
                self.filters_cache.insert(key.clone(), uids);
            }
        }

        self.libraries_cache.clear();
        for (key, library) in &self.manifest.libraries {
            let where_str = library.sql.as_ref().and_then(|s| s.where_.as_deref()).unwrap_or("1=1");
            let expanded_filter = expand_shorthand(where_str);
            let sql = format!("SELECT uid FROM albums WHERE {expanded_filter}");
            let mut stmt = self.conn.prepare(&sql)?;
            let uids: HashSet<u32> = stmt.query_map([], |row| row.get(0))?.filter_map(Result::ok).collect();
            self.libraries_cache.insert(key.clone(), uids);
        }

        self.orders_cache.clear();
        for (key, order) in &self.manifest.orders {
            let order_str = order.sql.order_by.as_deref().unwrap_or("uid ASC");
            let expanded_order = expand_shorthand(order_str);
            let sql = format!("SELECT uid FROM albums ORDER BY {expanded_order}");
            let mut stmt = self.conn.prepare(&sql)?;
            let uids: Vec<u32> = stmt.query_map([], |row| row.get(0))?.filter_map(Result::ok).collect();
            self.orders_cache.insert(key.clone(), uids);
        }

        self.shelves_cache.clear();
        for (key, shelf) in &self.manifest.shelves {
            if let Some(file_path) = &shelf.file {
                let expanded = crate::expand_path(file_path);
                if let Ok(content) = std::fs::read_to_string(&expanded) {
                    let lines: Vec<String> = content.lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();

                    if let Ok(json_arr) = serde_json::to_string(&lines) {
                        let sql = "SELECT a.uid FROM json_each(?1) j JOIN albums a ON a.id = j.value ORDER BY j.key";
                        if let Ok(mut stmt) = self.conn.prepare(sql) {
                            let uids: Vec<u32> = stmt.query_map([json_arr], |row| row.get(0))
                                .map(|rows| rows.filter_map(Result::ok).collect())
                                .unwrap_or_default();
                            self.shelves_cache.insert(key.clone(), uids);
                        }
                    }
                } else {
                    self.shelves_cache.insert(key.clone(), vec![]);
                }
            } else if let Some(sql_def) = &shelf.sql {
                let expanded_filter = expand_shorthand(sql_def.where_.as_deref().unwrap_or("1=1"));
                let order_clause = sql_def.order_by.as_deref().map(|o| format!(" ORDER BY {}", expand_shorthand(o))).unwrap_or_default();
                let sql = format!("SELECT uid FROM albums WHERE {expanded_filter}{order_clause}");
                if let Ok(mut stmt) = self.conn.prepare(&sql) {
                    let uids: Vec<u32> = stmt.query_map([], |row| row.get(0))
                        .map(|rows| rows.filter_map(Result::ok).collect())
                        .unwrap_or_default();
                    self.shelves_cache.insert(key.clone(), uids);
                }
            }
        }

        self.facets_cache.clear();
        for (key, grouper) in &self.manifest.groupers {
            let select_str = grouper.sql.select.as_deref().unwrap_or("''");
            let expanded_select = expand_shorthand(select_str);
            let sql = format!("SELECT uid, {expanded_select} FROM albums");
            let mut stmt = self.conn.prepare(&sql)?;
            let mut rows = stmt.query([])?;

            let mut map: HashMap<String, HashSet<u32>> = HashMap::new();

            while let Some(row) = rows.next()? {
                let uid: u32 = row.get(0)?;
                let raw_val: rusqlite::types::Value = row.get(1)?;

                let val_str = match raw_val {
                    rusqlite::types::Value::Text(s) => s,
                    rusqlite::types::Value::Integer(i) => i.to_string(),
                    rusqlite::types::Value::Real(f) => f.to_string(),
                    _ => continue,
                };

                if let Ok(Value::Array(arr)) = serde_json::from_str(&val_str) {
                    for v in arr {
                        if let Some(s) = v.as_str() {
                            map.entry(s.trim().to_string()).or_default().insert(uid);
                        }
                    }
                } else if let Ok(Value::String(s)) = serde_json::from_str(&val_str) {
                    map.entry(s.trim().to_string()).or_default().insert(uid);
                } else {
                    map.entry(val_str.trim().to_string()).or_default().insert(uid);
                }
            }
            self.facets_cache.insert(key.clone(), map);
        }

        Ok(())
    }

    pub fn request_view(&self, library: &str, library_filter: Option<&str>, sort: &str, filter_key: Option<&str>, filter_val: Option<&str>, reverse: bool) -> Vec<String> {
        let empty_set = HashSet::new();
        let library_mask = self.libraries_cache.get(library).unwrap_or(&empty_set);
        let mut final_mask = library_mask.clone();

        if let Some(lf) = library_filter
            && let Some(f_mask) = self.filters_cache.get(lf)
        {
            final_mask.retain(|uid| f_mask.contains(uid));
        }

        if let (Some(fk), Some(fv)) = (filter_key, filter_val) {
            if fk == "search" {
                let sql = "SELECT uid FROM albums WHERE {$.album.album} LIKE ?1 OR {$.album.albumartist} LIKE ?1";
                if let Ok(mut stmt) = self.conn.prepare(&expand_shorthand(sql)) {
                    let pattern = format!("%{fv}%");
                    if let Ok(match_uids_iter) = stmt.query_map([pattern], |row| row.get::<_, u32>(0)) {
                        let match_uids: HashSet<u32> = match_uids_iter.filter_map(Result::ok).collect();
                        final_mask.retain(|uid| match_uids.contains(uid));
                    }
                }
            } else if let Some(facet_vals) = self.facets_cache.get(fk) {
                if let Some(facet_mask) = facet_vals.get(fv) {
                    final_mask.retain(|uid| facet_mask.contains(uid));
                } else {
                    final_mask.clear();
                }
            }
        }

        let empty_vec = Vec::new();
        let sorted_uids = self.orders_cache.get(sort).unwrap_or(&empty_vec);

        let mut res: Vec<String> = sorted_uids.iter()
            .filter(|uid| final_mask.contains(*uid))
            .filter_map(|uid| self.uid_to_id.get(uid).cloned())
            .collect();

        if reverse {
            res.reverse();
        }
        res
    }

    pub fn request_shelf_view(&self, shelf_key: &str) -> Vec<String> {
        let empty_vec = Vec::new();
        let uids = self.shelves_cache.get(shelf_key).unwrap_or(&empty_vec);
        uids.iter().filter_map(|uid| self.uid_to_id.get(uid).cloned()).collect()
    }

    pub fn request_group(&self, library: &str, library_filter: Option<&str>, grouper: &str) -> Vec<Value> {
        let empty_set = HashSet::new();
        let library_mask = self.libraries_cache.get(library).unwrap_or(&empty_set);
        let mut final_mask = library_mask.clone();

        if let Some(lf) = library_filter
            && let Some(f_mask) = self.filters_cache.get(lf)
        {
            final_mask.retain(|uid| f_mask.contains(uid));
        }

        let mut results = Vec::new();
        if let Some(facet_map) = self.facets_cache.get(grouper) {
            for (val, mask) in facet_map {
                let count = mask.intersection(&final_mask).count();
                if count > 0 {
                    results.push(json!({
                        "value": val,
                        "label": val,
                        "count": count
                    }));
                }
            }
        }

        results.sort_by(|a, b| {
            let label_a = a.get("label").and_then(Value::as_str).unwrap_or("").to_lowercase();
            let label_b = b.get("label").and_then(Value::as_str).unwrap_or("").to_lowercase();
            alphanumeric_sort::compare_str(&label_a, &label_b)
        });

        results
    }

    pub fn get_album_json(&self, id: &str) -> Option<String> {
        let mut stmt = self.conn.prepare("SELECT metadata FROM albums WHERE id = ?1").ok()?;
        let mut rows = stmt.query([id]).ok()?;
        if let Some(row) = rows.next().ok()? {
            return row.get(0).ok();
        }
        None
    }
}
