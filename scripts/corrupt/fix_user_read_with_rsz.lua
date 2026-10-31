local steamid = ui.ask_integer("Enter your steamid")
local user_path = ui.open_file("Select Your Corrupted User Save File")

if user_path == nil then
	return
end

local user_save = SaveFile.read_native_fields(user_path, {0xdbe3f199, 0x85e904c1}, {"app.savedata.cUserSaveData", "via.storage.saveService.SaveFileDetail"}, steamid, game.MHWILDS)

local output_path = ui.save_file("Select the file to save the fixed file to")
if output_path == nil then
	print("Failed to receive output file location")
	return
end
print("Saving Fixed Save to " .. output_path)
user_save:save(output_path, steamid)
