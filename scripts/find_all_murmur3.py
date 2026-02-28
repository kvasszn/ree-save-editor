import mmh3
import sys
import re
f = open(sys.argv[1], 'r', encoding='latin-1')
#if len(sys.argv) < 3 and sys.argv[2]:
#    check = int(sys.argv[2], 16)

to_check = [524057212, 1106175613]#, 0xdbe3f199, 0x85e904c1, 0xb6182d04, 0xc2468a16, 0xbc922b61, 0x27945a5a, 0xe137773c, 0x1b5a30c0]
strings = f.readlines()
#for string in strings:
#    string = string.strip()
#    s = string.split('.')
#    for sub in s:
#        strings.append(sub)
for s in ["Player01", "Player02", "Player03", "Player04", "Player06", "Player07", "Player08", "Player09", "Player10", "Player11", "Player12", "Player13", "Player14", "Player15", "Player16"]:
    strings.append(s.lower())
    strings.append(s.upper())
    strings.append(s)



f = open(sys.argv[2], 'r', encoding='latin-1')
raw_text = f.read()

# Find all "words" (strings of letters/numbers)
# \w+ matches any alphanumeric character sequence
all_words = re.findall(r'\w+', raw_text)

# Convert to set for uniqueness, then back to a list
unique_strings = sorted(list(set(all_words)))

strings = strings + unique_strings

strings.append("detail")
strings.append("Detail")
strings.append("_Detail")
strings.append("_detail")
strings.append("__detail")
strings.append("__Detail")
strings.append("SaveData")
strings.append("SaveFile")
strings.append("SaveFileDetail")
strings.append("SaveFileDetail")

strings=set(strings)
hashes = {}
for i, string in enumerate(strings):
    h = mmh3.hash(string.strip(), 0xffffffff) & 0xffffffff
    if h in to_check:
        print(f"hashes equal for {hex(h)}: {string.strip()}")
    s = "_" + string.strip()
    h = mmh3.hash(s, 0xffffffff) & 0xffffffff
    if h in to_check:
        print(f"hashes equal for {hex(h)}: {s}")
    s = string.strip()[2:]
    h = mmh3.hash(s, 0xffffffff) & 0xffffffff
    if h in to_check:
        print(f"hashes equal for {hex(h)}: {s}")
    s = string.strip().lower()
    h = mmh3.hash(s, 0xffffffff) & 0xffffffff
    if h in to_check:
        print(f"hashes equal for {hex(h)}: {s}")
    s = string.strip().upper()
    h = mmh3.hash(s, 0xffffffff) & 0xffffffff
    if h in to_check:
        print(f"hashes equal for {hex(h)}: {s}")
    #if i % 10000 == 0 and i != 0:
    #    print(i)
    #hashes[h] = string.strip()
#
