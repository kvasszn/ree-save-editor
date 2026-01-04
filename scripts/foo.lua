print("CharacterData: " .. tostring(character_data))
character_data._Data[1]._BasicData.CharName = "foo"
local name = character_data._Data[1]._BasicData.CharName
if name then
    print("Found: " .. tostring(name))
else
    print("name not found")
end
