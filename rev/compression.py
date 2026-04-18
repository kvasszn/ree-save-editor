import zlib
import struct
import sys
print(sys.argv[1])
compressed_payload = open(sys.argv[1], 'rb').read()[0x18:]

def find_deflate_offset(data):
    # Try starting decompression from every possible byte offset
    for offset in range(len(data)):
        try:
            # Attempt to decompress raw DEFLATE starting at 'offset'
            decompressed = zlib.decompress(data[offset:], wbits=-15)
            zero_count = decompressed.count(b'\x00')
            if len(decompressed) > 0:
                zero_ratio = zero_count / len(decompressed)
                if zero_ratio >= 0.30:
                    print(f"✅ Success! Valid raw DEFLATE stream found starting at offset {offset}")
                    return decompressed
        except zlib.error:
            continue
            
    print("❌ Failed: Could not find a valid raw DEFLATE stream at any offset.")
    return None

def partial_decompress(data, start_offset=0):
    # Create a decompression object for raw DEFLATE
    decompressor = zlib.decompressobj(wbits=-15)
    valid_output = bytearray()
    
    print(f"Starting partial decompression at offset {start_offset}...")
    
    # Feed the data into the decompressor one byte at a time
    for i in range(start_offset, len(data)):
        try:
            # Pass a single byte slice; zlib handles the internal bit-buffering
            chunk = decompressor.decompress(data[i:i+1])
            valid_output.extend(chunk)
            if decompressor.eof:
                print(f"🎉 SUCCESS! Stream ended naturally at byte {i}.")
                print(f"Unused trailing data length: {len(data) - i - 1} bytes")
                break
            
        except zlib.error as e:
            print(f"💥 CRASH! zlib encountered an error at byte index {i}")
            print(f"   Bytes consumed before crash: {i - start_offset}")
            print(f"   zlib error message: {e}")
            break
            
    return bytes(valid_output)

import zlib

def brute_force_bit_flip(data, crash_index, window_size=80):
    start_idx = max(0, crash_index - window_size)
    print(f"Targeting compressed bytes {start_idx} to {crash_index}...")
    print("Flipping bits...")
    
    for byte_idx in range(start_idx, crash_index + 1):
        for bit_idx in range(8):
            # Create a mutable copy of the compressed payload
            test_data = bytearray(data)
            
            # Flip the specific bit using XOR
            test_data[byte_idx] ^= (1 << bit_idx)
            
            decompressor = zlib.decompressobj(wbits=-15)
            crashed = False
            
            try:
                # Try decompressing slightly PAST the known crash point
                decompressor.decompress(test_data[:crash_index + 100])
            except zlib.error:
                crashed = True
                
            # If we made it past the crash point without an error!
            if not crashed:
                print(f"\n🎉 BREAKTHROUGH! Flipped bit {bit_idx} at byte index {byte_idx}")
                print("   The stream bypassed the original crash point.")
                
                # Now try to decompress the entire repaired file
                try:
                    full_output = zlib.decompress(test_data, wbits=-15)
                    print(f"   ✅ FULL DECOMPRESSION SUCCESS!")
                    print(f"   Output buffer size: {hex(len(full_output))} bytes")
                    
                    # Save the repaired payload and the decompressed file
                    with open("repaired_payload.bin", "wb") as f:
                        f.write(test_data)
                    with open("fully_decompressed.bin", "wb") as f:
                        f.write(full_output)
                        
                    return test_data, full_output
                    
                except zlib.error as e:
                    print(f"   ⚠️ Stream progressed but crashed later: {e}")

    print("\n❌ No single bit-flip fixed the stream in this window.")
    return None

# Run it using the crash index you found earlier
# repaired_data = brute_force_bit_flip(compressed_payload, crash_index=39066, window_size=80)

print(hex(len(compressed_payload)))
#decompressed = find_deflate_offset(compressed_payload)
#decompressed = partial_decompress(compressed_payload)
repaired, decompressed = brute_force_bit_flip(compressed_payload, 39066, 0x1000)
recompressed_data = zlib.compress(decompressed, level=5, wbits=-15)
f = open('outputs/decompressed_idfk.bin', 'wb')
f.write(b"DSSS\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00")
f.write(decompressed)
#f.write(b"DSSS\x02\x00\x00\x00\x08\x00\x00\x00\x00\x00\x00\x00")
#f.write(int.to_bytes(len(recompressed_data) + 0x10, 8, byteorder='little'))
#f.write(int.to_bytes(1, 4, byteorder='little'))
#f.write(int.to_bytes(len(recompressed_data), 4, byteorder='little'))
#f.write(int.to_bytes(len(decompressed), 8, byteorder='little'))
#f.write(recompressed_data)
#raw_data = zlib.decompress(compressed_payload, wbits=-15)
#print(hex(len(raw_data)))
#for i in range(10):
#    recompressed_data = zlib.compress(raw_data, level=i, wbits=-15)
#    print(i, hex(len(recompressed_data)))
#    if len(recompressed_data) == len(compressed_payload):
#        print(compressed_payload == recompressed_data)
