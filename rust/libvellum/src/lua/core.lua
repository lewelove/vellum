_G.REGISTRY = {
    config = {},
    covers = {},
    keys = {
        album = {},
        tracks = {}
    },
    interfaces = {},
    actions = {},
    dependencies = {}
}

if not package.searchpath then
    package.searchpath = function(name, path)
        local sep = package.config:sub(1, 1)
        name = name:gsub("%.", sep)
        for c in path:gmatch("[^;]+") do
            local filename = c:gsub("%?", name)
            local f = io.open(filename, "r")
            if f then
                f:close()
                return filename
            end
        end
        return nil, "not found"
    end
end

local original_require = require
_G.require = function(modname)
    local path, _ = package.searchpath(modname, package.path)
    if path then
        REGISTRY.dependencies[path] = true
    end
    return original_require(modname)
end

local original_dofile = dofile
_G.dofile = function(filename)
    if filename then
        REGISTRY.dependencies[filename] = true
    end
    return original_dofile(filename)
end

local original_loadfile = loadfile
_G.loadfile = function(filename, mode, env)
    if filename then
        REGISTRY.dependencies[filename] = true
    end
    return original_loadfile(filename, mode, env)
end

function __VELLUM_GET_CONFIG() return REGISTRY.config end
function __VELLUM_GET_COVERS() return REGISTRY.covers end
function __VELLUM_GET_INTERFACES() return REGISTRY.interfaces end
function __VELLUM_GET_ACTIONS() return REGISTRY.actions end
function __VELLUM_GET_DEPENDENCIES()
    local deps = {}
    for path, _ in pairs(REGISTRY.dependencies) do table.insert(deps, path) end
    return deps
end
