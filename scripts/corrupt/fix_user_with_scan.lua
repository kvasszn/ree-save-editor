local steamid = ui.ask_integer("Enter your steamid")
local user_path = ui.open_file("Select Your Corrupted User Save File")

if user_path == nil then
	return
end

-- The empty save is encrypted with steamid 1
local fixed_save = fs.load_save("./assets/mhwilds/empty_user_save.bin", 1, game.MHWILDS)


print("Attempting to read all slots")
local recovered_slots = SaveFile.scan_n_objects(user_path, "app.savedata.cUserSaveParam", 3, steamid, game.MHWILDS)
print("Found " .. #recovered_slots .. " missing classes in corrupted save")

for i = 1, #recovered_slots do
	print("Copying Data from Missing Slot #" .. i)
	local slot = recovered_slots[i]
	for j = 1, #slot do
		local field_name = slot:field_name(j, game.MHWILDS)
		if field_name ~= nil then
			local ver_str = "_ver"
			if field_name:sub(- #ver_str) == ver_str then
				local good_ver = fixed_save[1]._Data[i][field_name]
				-- ewww but whatever
				if slot[field_name] > good_ver then
					print("Updating version for field " .. field_name .. ": " .. good_ver .. " -> " .. slot[field_name])
					fixed_save[1]._Data[i][field_name] = slot[field_name]
				elseif slot[field_name] < good_ver then
					print("Updating version for field " .. field_name .. ": " .. slot[field_name] .. " , " .. good_ver)
					fixed_save[1]._Data[i][field_name] = good_ver
				end
			else
				fixed_save[1]._Data[i][field_name] = slot[field_name]
			end
			-- for _ver fields, this should stay as the most recent one
		end
	end
end
local output_file = ui.save_file("Select the file to save the fixed file to")
if output_file == nil then
	print("Failed to receive output file location")
	return
end
print("Saving Fixed Save to " .. output_file)
fixed_save:save(output_file, steamid)
return
