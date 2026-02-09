local savefile = savefile
print("SaveFile: " .. tostring(savefile))
print(tostring(savefile._Data))
print(tostring(savefile._Data[1]))
print(tostring(savefile._Data[1].HunterId))
print(tostring(savefile._Data[2].HunterId))
savefile._Data[2].HunterId = "foobar"
savefile._Data[1] = savefile._Data[2]
savefile._Data[2].HunterId = "asldkasdklj"
print(tostring(savefile._Data[1].HunterId))
