import mmh3
import sys
f = open(sys.argv[1], 'r', encoding='latin-1')
#if len(sys.argv) < 3 and sys.argv[2]:
#    check = int(sys.argv[2], 16)

to_check = [0xdbe3f199, 0x85e904c1, 0xb6182d04, 0xc2468a16, 0xbc922b61, 0x27945a5a]
strings = f.readlines()
hashes = {}
for i, string in enumerate(strings):
    h = mmh3.hash(string.strip(), 0xffffffff) & 0xffffffff
    if h in to_check:
        print(f"hashes equal for {hex(h)}: {string.strip()}")
    #if i % 10000 == 0 and i != 0:
    #    print(i)
    #hashes[h] = string.strip()
#
