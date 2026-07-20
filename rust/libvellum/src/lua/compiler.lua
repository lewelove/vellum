_G.vl = _G.vl or {}
_G.vl.compile = {
    album = {
        key = function(t)
            local count = 0
            for _ in pairs(t) do count = count + 1 end
            if count > 1 then error("vl.compile.album.key({}) accepts only 1 key per call.") end
            for k, v in pairs(t) do
                if type(v) == "function" then
                    REGISTRY.keys.album[k] = v
                end
            end
        end,
        info = {
            date_added = function(func)
                REGISTRY.info.album.date_added = func
            end
        }
    },
    tracks = {
        key = function(t)
            local count = 0
            for _ in pairs(t) do count = count + 1 end
            if count > 1 then error("vl.compile.tracks.key({}) accepts only 1 key per call.") end
            for k, v in pairs(t) do
                if type(v) == "function" then
                    REGISTRY.keys.tracks[k] = v
                end
            end
        end
    }
}

_G.vl.compile.a = _G.vl.compile.album
_G.vl.compile.track = _G.vl.compile.tracks
_G.vl.compile.t = _G.vl.compile.tracks

_G.vl.compile.album.info.date_added(function(ctx, m)
    if m.system and m.system.album and m.system.album.system then
        local sys = m.system.album.system
        if sys.date_added then return sys.date_added end
        if sys.date_generated then return sys.date_generated end
    end
    error("No date_generated or date_added found in system.toml manifest")
end)

function __VELLUM_DISPATCHER(ctx, manifests)
    local results = { album = {}, tracks = {}, info = { album = {} } }
    
    if REGISTRY.info.album.date_added then
        local status, res = pcall(REGISTRY.info.album.date_added, ctx, manifests)
        if not status then
            error(string.format("Error evaluating vl.compile.album.info.date_added: %s", tostring(res)))
        end
        if res ~= nil and res ~= "" then
            results.info.album.date_added = res
        else
            error("vl.compile.album.info.date_added returned nil or empty")
        end
    else
        error("vl.compile.album.info.date_added is not defined")
    end

    for key_name, func in pairs(REGISTRY.keys.album) do
        local status, res = pcall(func, ctx, manifests)
        if not status then
            error(string.format("Error evaluating album key '%s': %s", key_name, res))
        end
        if res ~= nil and res ~= "" then
            results.album[key_name] = res
        end
    end
    
    for i = 1, ctx.total_tracks do
        results.tracks[i] = {}
        for key_name, func in pairs(REGISTRY.keys.tracks) do
            local status, res = pcall(func, ctx, manifests, i)
            if not status then
                error(string.format("Error evaluating track key '%s' at index %d: %s", key_name, i, res))
            end
            if res ~= nil and res ~= "" then
                results.tracks[i][key_name] = res
            end
        end
    end
    
    return results
end
