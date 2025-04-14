import json
import sys
import os


if len(sys.argv) == 1:
    print(
"""Incorrect arguments, must pass in path to Enums_Internal.hpp
    do something like: python3 gen/enumtojson.py <path/to/Enums_Internal.hpp> <outfile>
    <outfile> default is "enums.json"
    :D
""")
    exit(0)

enums_internal = sys.argv[1]
out_file = "enums.json"
if len(sys.argv) == 3:
    out_file = sys.argv[2]

if not os.path.exists(enums_internal):
    print(f"could not find enums_internal {enums_internal}")
    exit(0)
print(f"generating enum json from {enums_internal} to {out_file}")

# this code is bad but whatever
f = open(enums_internal, 'r', encoding="utf-8")
data = f.read()
tokens = data.split()
len = len(tokens)
i = 0

namespaces = {}
name = ""
enum_name = ""
enum_vals = {}

while i < len:
    if tokens[i] == "namespace":
        i += 1
        name = tokens[i]

    if tokens[i] == "enum":
        i += 1
        if tokens[i] == "class":
            i += 1
        enum_name = tokens[i]
        i += 1

    if  tokens[i+1] == "=":
        enum_id = tokens[i]
        i += 2
        enum_val = 0
        if tokens[i].startswith('0x'):
            enum_val = int(tokens[i].strip(","))
        else:
            enum_val = int(tokens[i].strip(","))
        enum_vals[enum_id] = str(enum_val)
        enum_vals[enum_val] = enum_id

    if tokens[i] == "};":
        i += 1
        full_name = name.replace("::", ".") + "." + enum_name
        namespaces[full_name] = enum_vals
        name = ""
        enum_name = ""
        enum_vals = {}

    i += 1

with open(out_file, "w") as file:
    json.dump(namespaces, file, indent=4) 
