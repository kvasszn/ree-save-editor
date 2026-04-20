import re
import sys
import os

def load_word_dict():
    """Loads the standard Linux dictionary into a set of words > 4 chars."""
    word_set = set()
    dict_path = "/usr/share/dict/rockyou.txt"
    
    if os.path.exists(dict_path):
        with open(dict_path, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                word = line.strip().lower()
                # We only care about words strictly greater than 4 characters
                if len(word) > 4:
                    word_set.add(word)
        print(f"Loaded {len(word_set)} words from {dict_path} for filtering.")
    else:
        print(f"Warning: {dict_path} not found.")
        print("To enable word filtering, run: sudo apt install wamerican (or equivalent)")
        
    return word_set

def contains_english_word(key_str, word_set):
    """Checks if any substring of length 5+ is a valid English word."""
    if not word_set:
        return False
        
    s_lower = key_str.lower()
    n = len(s_lower)
    
    # Check every possible substring from length 5 up to the full string length
    for length in range(5, n + 1):
        for i in range(n - length + 1):
            if s_lower[i:i+length] in word_set:
                return True
                
    return False

def extract_blowfish_keys(file_path):
    pattern = re.compile(br'\b[a-zA-Z0-9]{4,56}\b')
    word_set = load_word_dict()
    potential_keys = set()
    
    print(f"Scanning {file_path}...")
    
    try:
        with open(file_path, 'rb') as f:
            data = f.read()
            
            for match in pattern.finditer(data):
                key_str = match.group(0).decode('ascii')
                
                # 1. Entropy Filter
                has_upper = any(c.isupper() for c in key_str)
                has_lower = any(c.islower() for c in key_str)
                has_digit = any(c.isdigit() for c in key_str)
                
                if has_upper and has_lower and has_digit:
                    # 2. Dictionary Filter (The new addition!)
                    if not contains_english_word(key_str, word_set):
                        potential_keys.add(key_str)
                    
    except FileNotFoundError:
        print(f"Error: Could not find file {file_path}")
        return []

    return sorted(list(potential_keys), key=lambda x: (len(x), x))

if __name__ == "__main__":
    target_file = sys.argv[1] if len(sys.argv) > 1 else "game.exe"
    keys = extract_blowfish_keys(target_file)
    
    print(f"\nFound {len(keys)} highly probable keys:\n" + "-"*30)
    for key in keys:
        print(f"[{len(key):>2} chars] {key}")
