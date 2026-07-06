pub mod config;

use anyhow::{Context, Result};
use config::{AppConfig, CoversConfig, KeyConfig};
use indexmap::IndexMap;
use mlua::{Lua, LuaSerdeExt, Table};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const BOOTSTRAP: &str = r#"
local REGISTRY = {
    config = {},
    covers = {},
    keys = {},
    keys_order = {}
}

_G.vl = {
    compile = {}
}

function vl.config(t)
    REGISTRY.config = t
end

function vl.compile.cover(name, t)
    REGISTRY.covers[name] = t
end

function vl.compile.key(name, t)
    local user_func = t.output
    if user_func == nil then
        user_func = function(v) return v end
    end
    t.output = user_func
    if t.type == nil then t.type = "string" end
    REGISTRY.keys[name] = t
    table.insert(REGISTRY.keys_order, name)
end

function __VELLUM_GET_CONFIG()
    return REGISTRY.config
end

function __VELLUM_GET_COVERS()
    local res = {}
    for k, v in pairs(REGISTRY.covers) do
        res[k] = {
            interpolation = v.interpolation,
            size = v.size
        }
    end
    return res
end

function __VELLUM_GET_KEYS()
    local res = {}
    for _, name in ipairs(REGISTRY.keys_order) do
        local v = REGISTRY.keys[name]
        table.insert(res, {
            name = name,
            level = v.level,
            type = v.type,
            manifests = v.manifests,
            newline = v.newline or false,
        })
    end
    return res
end

function __VELLUM_DISPATCHER(ctx)
    local results = { album = {}, tracks = {} }
    
    for key_name, cfg in pairs(REGISTRY.keys) do
        if cfg.level == "album" then
            local raw_val = nil
            if ctx.album then raw_val = ctx.album[key_name] end
            
            local status, res = pcall(cfg.output, raw_val, ctx)
            if status then
                results.album[key_name] = res
            else
                print(string.format("Warning: Error evaluating album key '%s': %s", key_name, res))
                results.album[key_name] = nil
            end
        end
    end
    
    for i = 1, ctx.track_count do
        results.tracks[i] = {}
        for key_name, cfg in pairs(REGISTRY.keys) do
            if cfg.level == "track" then
                local raw_val = nil
                if ctx.tracks and ctx.tracks[i] then raw_val = ctx.tracks[i][key_name] end
                
                local status, res = pcall(cfg.output, raw_val, ctx, i)
                if status then
                    results.tracks[i][key_name] = res
                else
                    print(string.format("Warning: Error evaluating track key '%s' at index %d: %s", key_name, i, res))
                    results.tracks[i][key_name] = nil
                end
            end
        end
    end
    
    return results
end
"#;

pub struct LuaEngine {
    pub lua: Lua,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyConfigRaw {
    pub name: String,
    pub level: String,
    #[serde(rename = "type")]
    pub type_: crate::types::VellumDataType,
    pub manifests: Option<String>,
    #[serde(default)]
    pub newline: bool,
}

pub struct EvaluatedLuaData {
    pub app: AppConfig,
    pub covers: IndexMap<String, CoversConfig>,
    pub keys: IndexMap<String, KeyConfig>,
}

impl LuaEngine {
    pub fn new() -> Result<Self> {
        let lua = Lua::new();
        lua.load(BOOTSTRAP)
            .exec()
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Failed to load Lua bootstrap")?;
        Ok(Self { lua })
    }

    pub fn evaluate_config(&self, path: &Path) -> Result<EvaluatedLuaData> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read {}", path.display()))?;

        self.lua
            .load(&content)
            .set_name(path.to_string_lossy())
            .exec()
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Failed to execute vellum.lua")?;

        let globals = self.lua.globals();

        let get_config: mlua::Function = globals
            .get("__VELLUM_GET_CONFIG")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let config_table: Table = get_config.call(()).map_err(|e| anyhow::anyhow!("{e}"))?;
        let app_config: AppConfig = self
            .lua
            .from_value(mlua::Value::Table(config_table))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let get_covers: mlua::Function = globals
            .get("__VELLUM_GET_COVERS")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let covers_table: Table = get_covers.call(()).map_err(|e| anyhow::anyhow!("{e}"))?;
        let covers: IndexMap<String, CoversConfig> = self
            .lua
            .from_value(mlua::Value::Table(covers_table))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let get_keys: mlua::Function = globals
            .get("__VELLUM_GET_KEYS")
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let keys_table: Table = get_keys.call(()).map_err(|e| anyhow::anyhow!("{e}"))?;
        let keys_raw: Vec<KeyConfigRaw> = self
            .lua
            .from_value(mlua::Value::Table(keys_table))
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut keys = IndexMap::new();
        for k in keys_raw {
            keys.insert(
                k.name,
                KeyConfig {
                    level: k.level,
                    type_: k.type_,
                    manifests: k.manifests,
                    newline: k.newline,
                },
            );
        }

        Ok(EvaluatedLuaData {
            app: app_config,
            covers,
            keys,
        })
    }

    pub fn execute_dispatcher(&self, ctx_val: &serde_json::Value) -> Result<serde_json::Value> {
        let globals = self.lua.globals();
        let dispatcher: mlua::Function = globals
            .get("__VELLUM_DISPATCHER")
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let lua_ctx = self
            .lua
            .to_value(ctx_val)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let res: mlua::Table = dispatcher.call(lua_ctx).map_err(|e| anyhow::anyhow!("{e}"))?;

        let json_res: serde_json::Value = self
            .lua
            .from_value(mlua::Value::Table(res))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(json_res)
    }
}

#[must_use]
pub fn resolve_config_path() -> Option<PathBuf> {
    if let Some(home_config) = dirs::home_dir().map(|h| h.join(".config/vellum/vellum.lua"))
        && home_config.exists()
    {
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
        let local_nested = curr.join("config/vellum.lua");
        if local_nested.exists() {
            return Some(local_nested);
        }

        let local_root = curr.join("vellum.lua");
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

#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub app: AppConfig,
    pub covers: IndexMap<String, CoversConfig>,
    pub keys: IndexMap<String, KeyConfig>,
    pub path: PathBuf,
}

impl ResolvedConfig {
    pub fn load() -> Result<Self> {
        let path = resolve_config_path().context("Could not locate vellum.lua")?;
        let engine = LuaEngine::new()?;
        let mut evaluated = engine.evaluate_config(&path)?;

        if evaluated.covers.is_empty() {
            evaluated.covers.insert(
                "master".to_string(),
                CoversConfig {
                    interpolation: Some("mitchell".to_string()),
                    size: 1080,
                },
            );
            evaluated.covers.insert(
                "thumbnail".to_string(),
                CoversConfig {
                    interpolation: Some("lanczos".to_string()),
                    size: 190,
                },
            );
        } else if !evaluated.covers.contains_key("master") {
            evaluated.covers.insert(
                "master".to_string(),
                CoversConfig {
                    interpolation: Some("mitchell".to_string()),
                    size: 1080,
                },
            );
        }

        Ok(Self {
            app: evaluated.app,
            covers: evaluated.covers,
            keys: evaluated.keys,
            path,
        })
    }
}

thread_local! {
    pub static LUA_VM: RefCell<Option<LuaEngine>> = const { RefCell::new(None) };
}

pub fn get_or_init_lua_vm<F, R>(config_path: &Path, f: F) -> Result<R>
where
    F: FnOnce(&LuaEngine) -> Result<R>,
{
    LUA_VM.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            let engine = LuaEngine::new()?;
            engine.evaluate_config(config_path)?;
            *opt = Some(engine);
        }
        f(opt.as_ref().unwrap())
    })
}
