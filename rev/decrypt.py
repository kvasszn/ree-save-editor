import mmh3
file = "../save-files/wilds/ornd07-netizen/MHWilds_Save/data001Slot.bin"

data = open(file, 'rb').read()
data, e = data[16:-16], data[-4:]
h = mmh3.hash(data, 0xffffffff, signed=False)
print(hex(h))
print(hex(int.from_bytes(e, byteorder='little')))

