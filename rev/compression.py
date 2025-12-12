import zlib
import struct
import sys
print(sys.argv[1])
compressed_payload = open(sys.argv[1], 'rb').read()[0x18:]

print(hex(len(compressed_payload)))
raw_data = zlib.decompress(compressed_payload, wbits=-15)
print(hex(len(raw_data)))
for i in range(10):
    recompressed_data = zlib.compress(raw_data, level=i, wbits=-15)
    print(i, hex(len(recompressed_data)))
    if len(recompressed_data) == len(compressed_payload):
        print(compressed_payload == recompressed_data)
