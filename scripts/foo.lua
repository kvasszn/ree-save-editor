local savefile = savefile
local save_data = savefile[1]._Data
print("SaveFile: " .. tostring(savefile))
print(tostring(savefile[1]._Data[1].HunterId))
print(tostring(savefile[1]._Data[2].HunterId))
savefile[1]._Data[2].HunterId = "ASSD"
print(save_data[1].HunterId)
print(#save_data)
save_data[1].HunterId = "IDK"
print(tostring(savefile[1]._Data[2].HunterId))
