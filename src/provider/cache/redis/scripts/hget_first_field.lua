--[[
hget_first_field.lua
====================

Requirements:
-------------
Redis: v2.6.0 or higher
Lua: v5.1.0 or higher

Description:
------------
Finds and returns the **first available field** in a Redis hash stored at a given key
from a provided list of field names.

Arguments:
----------
KEYS[1] : Redis key containing the hash.
ARGV    : List of field names to check, in priority order.

Return:
-------
If one of the fields exists, returns:
    { <index>, <value> }
Where:
    - <index> is the 0-based position of the found field in the ARGV list.
    - <value> is the corresponding hash field value.

If none of the provided fields exist, returns:
    false
]]--

local key = KEYS[1]
local n = #ARGV

for i = 1, n do
    local v = redis.call('HGET', key, ARGV[i])
    if v ~= false and v ~= nil then
        -- Return index (0-based) and value
        return {i - 1, v}
    end
end

-- None of the fields were available
return false