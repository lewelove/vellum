_G.vl = _G.vl or {}
_G.vl.config = function(t)
    if t.interfaces then
        for k, v in pairs(t.interfaces) do
            if type(v) == "boolean" then
                if v == true then
                    REGISTRY.interfaces[k] = { enable = true, config = {}, assets = {} }
                end
            elseif type(v) == "table" then
                v.enable = true
                if v.config == nil then v.config = {} end
                if v.assets == nil then v.assets = {} end
                REGISTRY.interfaces[k] = v
            end
        end
        t.interfaces = nil
    end
    REGISTRY.config = t
end

_G.vl.interfaces = function(t)
    for k, v in pairs(t) do
        if type(v) == "boolean" then
            if v == true then
                REGISTRY.interfaces[k] = { enable = true, config = {}, assets = {} }
            end
        elseif type(v) == "table" then
            v.enable = true
            if v.config == nil then v.config = {} end
            if v.assets == nil then v.assets = {} end
            REGISTRY.interfaces[k] = v
        end
    end
end

_G.vl.actions = function(t)
    for k, v in pairs(t) do
        if type(v) == "boolean" then
            if v == true then
                REGISTRY.actions[k] = { config = {} }
            end
        elseif type(v) == "table" then
            if v.config == nil then v.config = {} end
            REGISTRY.actions[k] = v
        end
    end
end

_G.vl.compiler = _G.vl.compiler or {}
_G.vl.compiler.covers = function(name, t)
    REGISTRY.covers[name] = t
end
