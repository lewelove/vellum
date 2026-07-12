local REGISTRY = {
    config = {},
    covers = {},
    keys = {},
    keys_order = {},
    interfaces = {}
}

_G.vl = {
    config = function(t)
        if t.interfaces then
            for k, v in pairs(t.interfaces) do
                if type(v) == "boolean" then
                    if v == true then
                        REGISTRY.interfaces[k] = { enable = true, config = {} }
                    end
                elseif type(v) == "table" then
                    v.enable = true
                    if v.config == nil then
                        v.config = {}
                    end
                    REGISTRY.interfaces[k] = v
                end
            end
            t.interfaces = nil
        end
        REGISTRY.config = t
    end,
    interfaces = function(t)
        for k, v in pairs(t) do
            if type(v) == "boolean" then
                if v == true then
                    REGISTRY.interfaces[k] = { enable = true, config = {} }
                end
            elseif type(v) == "table" then
                v.enable = true
                if v.config == nil then
                    v.config = {}
                end
                REGISTRY.interfaces[k] = v
            end
        end
    end,
    compiler = {
        covers = function(name, t)
            REGISTRY.covers[name] = t
        end
    }
}

local function normalize_level(lvl)
    if lvl == "album" or lvl == "a" then
        return "album"
    elseif lvl == "track" or lvl == "t" or lvl == "tracks" then
        return "track"
    end
    return nil
end

local key_meta = {
    __index = function(self, raw_lvl)
        local level = normalize_level(raw_lvl)
        if not level then
            return nil
        end
        return function(defs)
            for key_name, t in pairs(defs) do
                if type(t) == "table" then
                    if t.type == nil then
                        t.type = "object"
                    end
                    if t.required == nil then
                        t.required = false
                    end
                    t.level = level
                    REGISTRY.keys[key_name] = t
                    table.insert(REGISTRY.keys_order, key_name)
                end
            end
        end
    end
}

_G.vl.compiler.keys = setmetatable({}, key_meta)

local function get_album_raw_val(ctx, key_name)
    if ctx.album and ctx.album[key_name] ~= nil then
        return ctx.album[key_name]
    end
    if ctx.tracks and #ctx.tracks > 0 then
        local first_val = ctx.tracks[1][key_name]
        if first_val == nil then return nil end
        for i = 2, #ctx.tracks do
            if ctx.tracks[i][key_name] ~= first_val then
                return nil
            end
        end
        return first_val
    end
    return nil
end

local function get_track_raw_val(ctx, idx, key_name)
    if ctx.tracks and ctx.tracks[idx] and ctx.tracks[idx][key_name] ~= nil then
        return ctx.tracks[idx][key_name]
    end
    if ctx.album and ctx.album[key_name] ~= nil then
        return ctx.album[key_name]
    end
    return nil
end

function __VELLUM_GET_CONFIG()
    return REGISTRY.config
end

function __VELLUM_GET_COVERS()
    return REGISTRY.covers
end

function __VELLUM_GET_INTERFACES()
    return REGISTRY.interfaces
end

function __VELLUM_GET_KEYS()
    local res = {}
    for _, name in ipairs(REGISTRY.keys_order) do
        local v = REGISTRY.keys[name]
        table.insert(res, {
            name = name,
            level = v.level,
            type = v.type
        })
    end
    return res
end

function __VELLUM_DISPATCHER(ctx)
    local results = { album = {}, tracks = {} }
    
    for _, key_name in ipairs(REGISTRY.keys_order) do
        local cfg = REGISTRY.keys[key_name]
        if cfg.level == "album" then
            local raw_val = get_album_raw_val(ctx, key_name)
            
            if cfg.required and raw_val == nil then
                error(string.format("Compile error: Required album key '%s' is nil", key_name))
            end
            
            if type(cfg.output) == "function" then
                local status, res = pcall(cfg.output, raw_val, ctx)
                if not status then
                    error(string.format("Error evaluating album key '%s': %s", key_name, res))
                end
                if res == nil and cfg.required then
                    error(string.format("Required album key '%s' resolved to nil", key_name))
                end
                if res ~= nil then
                    results.album[key_name] = res
                end
            else
                if raw_val ~= nil then
                    results.album[key_name] = raw_val
                end
            end
        end
    end
    
    for i = 1, ctx.track_count do
        results.tracks[i] = {}
        for _, key_name in ipairs(REGISTRY.keys_order) do
            local cfg = REGISTRY.keys[key_name]
            if cfg.level == "track" then
                local raw_val = get_track_raw_val(ctx, i, key_name)
                
                if cfg.required and raw_val == nil then
                    error(string.format("Compile error: Required track key '%s' at index %d is nil", key_name, i))
                end
                
                if type(cfg.output) == "function" then
                    local status, res = pcall(cfg.output, raw_val, ctx, i)
                    if not status then
                        error(string.format("Error evaluating track key '%s' at index %d: %s", key_name, i, res))
                    end
                    if res == nil and cfg.required then
                        error(string.format("Required track key '%s' at index %d resolved to nil", key_name, i))
                    end
                    if res ~= nil then
                        results.tracks[i][key_name] = res
                    end
                else
                    if raw_val ~= nil then
                        results.tracks[i][key_name] = raw_val
                    end
                end
            end
        end
    end
    
    return results
end
