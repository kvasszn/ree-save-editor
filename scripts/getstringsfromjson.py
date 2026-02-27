import re
import sys
f = open(sys.argv[1], 'r', encoding='latin-1')
raw_text = f.read()

# Find all "words" (strings of letters/numbers)
# \w+ matches any alphanumeric character sequence
all_words = re.findall(r'\w+', raw_text)

# Convert to set for uniqueness, then back to a list
unique_strings = sorted(list(set(all_words)))

print(unique_strings)
