import json
import sys

il2cpp_path = sys.argv[1]
f = open(il2cpp_path, 'r')
il2cpp = json.load(f)


hash_classes = {}

for class_name, class_ in il2cpp.items():
    if class_name.endswith("ID.Hash"):
        hash_classes[class_name] = {}
        for field_name, field  in class_["fields"].items():
            if "Static" in field["flags"]:
                value = field["default"]
                hash_classes[class_name][field_name] = str(value)
                hash_classes[class_name][value] = field_name

f = open(sys.argv[2], 'w')
json.dump(hash_classes, f, indent=4)
