import json
import sys

def binary_to_json(binary_path, json_path):
    with open(binary_path, 'rb') as f:
        byte_data = list(f.read())  # Read as bytes and convert to list of ints

    with open(json_path, 'w') as f:
        json.dump(byte_data, f)

print(sys.argv)
binary_to_json(sys.argv[1], sys.argv[1] + ".json")
