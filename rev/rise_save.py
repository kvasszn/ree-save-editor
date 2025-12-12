import sys
import struct
import mmh3
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.backends import default_backend
import os

save_file_path = sys.argv[1]
save_file_path = "/home/nikola/.local/share/Steam/userdata/292073414/1446780/remote/win64_save/data00-1.bin"
f = open(save_file_path, 'rb')
data = f.read()

header = data[0:16]
header = struct.unpack('<4I', header)
end = data[-12:]
decrypted_size, checksum_target = struct.unpack('<QI', end)
print([hex(x) for x in header])
print(hex(decrypted_size), hex(checksum_target))

checksum = mmh3.hash(data[:-4], 0xffffffff)
print(hex(checksum))

def calculate_table_offset(param_1):
    param_1_low32 = param_1 & 0xFFFFFFFF
    uVar1 = (param_1_low32 * 5) // 3
    byte3 = (uVar1 >> 24) & 0xFF
    byte2 = (uVar1 >> 16) & 0xFF
    byte1 = (uVar1 >> 8) & 0xFF
    byte0 = uVar1 & 0xFF
    bVar3 = (byte3 + byte2 + byte1 + byte0) & 0x7F
    lVar2 = bVar3 * 0x180
    return lVar2

param1 = 0x475b000 + 0x40320
print(hex(param1))
x = calculate_table_offset(param1)
table1 = 0x14f96a430 
table2 = 0x14f96a330 
a = table1 + x
b = table2 + x

data_14f96b4b0 = [ 0x40, 0x4f, 0xc2, 0xca, 0x2f, 0x1e, 0xb5, 0x5f, 0xaf, 0xb4, 0x96, 0x4f, 0x01, 0x00]
int_14f96b4b0 = int.from_bytes(data_14f96b4b0, byteorder='little')
int_14f96b3b0 = 0x14F96B3AD6CD70D6AA3302DA0

print(hex(int_14f96b4b0))
print(hex(a))
bigint1 = (int_14f96b4b0 // a) ^ 0xffffffffffffffff
bigint2 = (int_14f96b3b0 // b) ^ 0xffffffffffffffff
print(hex(bigint1))
print(hex(bigint2))
local_228 = bigint2
print(hex(0x14f96a2f0 + 0x40320))


