import struct
from Crypto.Cipher import Blowfish

def decrypt(key: bytes, ciphertext: bytes, little_endian_quirk: bool = True) -> bytes:
    if not little_endian_quirk:
        cipher = Blowfish.new(key, Blowfish.MODE_CBC, b'\x00' * 8)
        return cipher.decrypt(ciphertext)

    cipher = Blowfish.new(key, Blowfish.MODE_ECB)
    iv_l = iv_r = 0
    plaintext = bytearray()

    for i in range(0, len(ciphertext), 8):
        block = ciphertext[i:i+8]
        if len(block) < 8:
            break
            
        cl, cr = struct.unpack('<II', block)
        
        dec = cipher.decrypt(struct.pack('>II', cl, cr))
        pl, pr = struct.unpack('>II', dec)
        
        pl ^= iv_l
        pr ^= iv_r
        
        iv_l, iv_r = cl, cr
        
        plaintext.extend(struct.pack('<II', pl, pr))
        
    return bytes(plaintext)

import sys
path = sys.argv[1]
f = open(path, 'rb')
data = f.read()
ct = data[16:]
key = b"K<>$cl%isqA|~nV4W5~3z_Q)j]5DHdB9sb{cI9Hn&Gqc-zO8O6zf"
a = decrypt(key, ct[0:8]) # DSSS
b = decrypt(key, ct[8:16]) # steamid32
print(a.hex(), b.hex())
pt = decrypt(key, ct[16:])
print(pt)
f = open('decrypted.bin', 'wb')
f.write(pt)
