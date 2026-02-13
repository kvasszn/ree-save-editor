local steamid = ui.ask_integer("Enter your steamid")
local system_path = ui.open_file("Select System Save File")
local user_path = ui.open_file("Select User Save File")

if user_path == nil then
	return
end

local user_save = fs.load_save(user_path, steamid, game.MHWILDS)

if system_path == nil then
	return
end
local system_save = fs.load_save(system_path, steamid, game.MHWILDS)

system_save[1]._Data._SystemCommon.HunterTicketsUsed:write_u64(0, 0);
system_save[1]._Data._SystemCommon.PalicoTicketsUsed:write_u64(0, 0);

local slots = user_save[1]._Data
if slots ~= nil then
	for i = 1, #slots do
		slots[i]._FreeBuffer.BufferInt[18] = 0
		slots[i]._FreeBuffer.BufferInt[19] = 0
	end
end

system_save:save("./outputs/saves/reset_tickets/data00-1.bin", steamid)
user_save:save("./outputs/saves/reset_tickets/data001Slot.bin", steamid)
