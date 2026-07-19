_G.vl = _G.vl or {}
_G.vl.fn = {
    require = function(v)
        if v == nil or v == "" then error("Value is required") end
        return v
    end,
    type_check = function(v, t)
        if v == nil or v == "" then return v end
        local lua_type = type(v)
        if t == "string" or t == "datetime" or t == "path" or t == "url" then
            if lua_type ~= "string" then error("Expected " .. t .. " but got " .. lua_type) end
        elseif t == "integer" or t == "float" or t == "number" then
            if lua_type ~= "number" then error("Expected " .. t .. " but got " .. lua_type) end
        elseif t == "boolean" then
            if lua_type ~= "boolean" then error("Expected boolean but got " .. lua_type) end
        elseif t == "array" or t == "object" or t == "list" then
            if lua_type ~= "table" then error("Expected table but got " .. lua_type) end
        end
        return v
    end
}
