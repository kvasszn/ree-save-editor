import mmh3
import sys
f = open(sys.argv[1])
#if len(sys.argv) < 3 and sys.argv[2]:
#    check = int(sys.argv[2], 16)
check = 0x861ab707
#print(hex(mmh3.hash("_MaxElement", 0xffffffff) & 0xffffffff))


to_check = [0xdbe3f199, 0x85e904c1]
strings = f.readlines()
hashes = {}
for string in strings:
    h = mmh3.hash(string.strip(), 0xffffffff) & 0xffffffff
    if h in to_check or h == check:
        print(f"hashes equal for {hex(h)}: {string.strip()}")
    #hashes[h] = string.strip()

