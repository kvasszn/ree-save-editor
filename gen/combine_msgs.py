import json
import os
import sys

def combine_json_files(input_folder, output_file, extension):
    combined_data = {"msgs": {}, "name_to_uuid": {}}
    for root, dirs, files in os.walk(input_folder):
        for name in files:
            if name.endswith(extension):
                filepath = os.path.join(root, name)
                try:
                    # Load JSON data
                    with open(filepath, 'r', encoding='utf-8') as f:
                        data = json.load(f)
                        # Merge JSON content
                        combined_data['msgs'].update(data['msgs'])
                        combined_data['name_to_uuid'].update(data['name_to_uuid'])
                except Exception as e:
                    print(f"Error processing file {filepath}: {e}")

    # Write combined data to the output file
    with open(output_file, 'w') as f:
        json.dump(combined_data, f, indent=4, ensure_ascii=False)

    print(f"Combined JSON saved to {output_file}")

version = sys.argv[1]
folder = sys.argv[2]
print(version, folder)
version = 23
extension = f"msg.{version}.json"
combine_json_files(folder, os.path.join(folder, "combined_msgs.json"), extension)

