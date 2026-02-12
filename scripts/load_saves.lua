local steamid = 76561198252339142
local system_path = "~/.local/share/Steam/userdata/292073414/2246340/remote/win64_save/data00-1.bin"
local user_path = "~/.local/share/Steam/userdata/292073414/2246340/remote/win64_save/data001Slot.bin"
---local steamid = ui.ask_integer("Enter your steamid")
---local system_path = ui.open_file("Select System Save File")
---local user_path = ui.open_file("Select User Save File")

if user_path == nil then
	return
end

local user_save = fs.load_save(user_path, steamid, game.MHWILDS)

if system_path == nil then
	return
end
local system_save = fs.load_save(system_path, steamid, game.MHWILDS)

print("Name: " .. user_save[1]._Data[1]._BasicData.CharName)
user_save[1]._Data[2]._BasicData.CharName = "foobarbaz"

system_save:save("./outputs/saves/modified_system2.bin", steamid)
user_save:save("./outputs/saves/modified_user2.bin", steamid)
