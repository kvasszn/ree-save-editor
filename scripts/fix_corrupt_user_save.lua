local steamid = ui.ask_integer("Enter your steamid")
local user_path = ui.open_file("Select Your Corrupted User Save File")

-- local output_path = "./outputs/fixed/data001Slot.bin"

if user_path == nil then
    return
end

-- The empty save is encrypted with steamid 1
local fixed_save = fs.load_save("./assets/mhwilds/empty_user_save.bin", 1, game.MHWILDS)

print("Attempting to read missing classes first")
local user_missing = SaveFile.scan_missing(user_path, steamid, game.MHWILDS)
print("Found " .. #user_missing .. " missing classes in corrupted save")
if #user_missing ~= 0 then
    print("Copying Data from Missing Classes")
    local user_slot = user_missing[1]._Data[1]
    fixed_save[1]._Data[1] = user_slot

    local output_file = ui.save_file("Select the file to save the fixed file to")
    if output_file == nil then
        print("Failed to receive output file location")
        return
    end
    print("Saving Fixed Save to ".. output_file)
    fixed_save:save(output_file, steamid)
    return
end

local user_save = SaveFile.scan_classes(user_path, steamid, game.MHWILDS)
print("Found " .. #user_save .. " classes in corrupted save")
if #user_save ~= 0 then
    print("Copying First Scanned Class (Hopefully it's correct, make sure in the editor after)")
    local user_slot = user_save[1]
    fixed_save[1]._Data[1] = user_slot

    local output_file = ui.save_file("Select the file to save the fixed file to")
    if output_file == nil then
        print("Failed to receive output file location")
        return
    end
    print("Saving Fixed Save to ".. output_file)
    fixed_save:save(output_file, steamid)
    return
end
