import sys

with open(sys.argv[1], 'rb') as f:
    data = f.read()

aob = bytes.fromhex("4531c049b9157c4a7fb979379e49bab9e5e41c6d4758bf49bbeb113113bb49d094")

offset = data.find(aob)

if offset != -1:
    seed_bytes = data[offset-8:offset]
    seed = int.from_bytes(seed_bytes, byteorder='little')
    print(f"Found seed {seed:#x}")

else:
    print("Could not find aob")
