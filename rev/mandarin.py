M = int.from_bytes(bytes.fromhex("f33b6fb972a0b72515e45c391829e182ad8a9bdc0a64d3444d79c810ab863717"), byteorder='little')
P2 = int.from_bytes(bytes.fromhex("f99db75c39d0db920a72ae1c8c9470c156c54d6e05b269a2a63c648855c39b0b"), byteorder='little')
A = int.from_bytes(bytes.fromhex("a1002346d9d854e5c18e4ce4eb641bed7d282226c7268648c1909d59abfa7215"), byteorder='little')
K = int.from_bytes(bytes.fromhex("395097eefeffeffe000000000000000000000000000000000000000000000000"), byteorder='little')

#result = (K * P2) - P2 = K * P2 (% P2)

B = int.from_bytes(bytes.fromhex("e66f544afcce68c5ef07b9a07b277585344a1db61376e831f73b9fbd5f44f715"), byteorder='little')

e = 0x14
result1 = pow(A, e, M)
print(f"A ^ e % M = {result1.to_bytes(32, 'little').hex()}")
result2 = pow(B, e, M)
print(f"B ^ e % M = {result2.to_bytes(32, 'little').hex()}")

result3 = pow(P2, e, M)
print(f"P2 ^ e % M = {result3.to_bytes(32, 'little').hex()}")

#mul = pow(B, e, M)
mul = result1
e = 0x14
print(f"{mul.to_bytes(32, 'little').hex()}")
print("pls", hex((M - 1) // P2))

key_iv = bytes.fromhex("f6037f94fb38f8cd37b145deda249d793a9fb516119373e5f74d91d5469750eb")
#key_iv=bytes.fromhex("f2a746e314d5ea52de15a16ebccda85646ad620cdffbcfff7c660eb536652f6e")
for i in range(4):
    block = int.from_bytes(key_iv[i*8:i*8+8], byteorder='little')
    print(key_iv[i*8:i*8+8].hex())
    result = block*mul
    print(f"result={result.to_bytes(64, 'little').hex()}")
    print("mul=", hex(mul))
    print("result=", hex(result))

result = int.from_bytes(bytes.fromhex("1050499fd299fc71c5b532a9f0393c75b9d4f7a4d2e691ae5d30d44663c28b205aa08e0942d89a05000000000000000000000000000000000000000000000000"), byteorder='little')
chunk = int.from_bytes(bytes.fromhex("d48c8f1040aebfe8"), byteorder='little')
print(f"mul_recovered={((result//chunk | mul)).to_bytes(32, 'little').hex()}")



result = int.from_bytes(bytes.fromhex("c4c31dd51fa17e739101d0026194f7a25158e1891087210b5e1125aebe8881dbd9b568af03d24602000000000000000000000000000000000000000000000000"), byteorder='little')
chunk = int.from_bytes(bytes.fromhex("e5cbf201d7538a5e"), byteorder='little')
print(hex(result // chunk))

# the rsa stuff?
key = 76561198252339142
e = 65537
n = int.from_bytes(b'\x4f\xa4\x48\x36\x4f\x5b\x35\x07\xe9\x45\x07\x5c\xc2\x19\x94\xbd\xed\xef\x96\x96\x2c\x74\xd5\x31\x59\xd5\x0a\x5c\x62\xed\x50\x86\x48\x85\xdd\xfe\x79\x70\x5d\xfa\xd0\xb6\x38\x22\x0c\xa2\x29\x9f\xcc\xae\x15\x21\x64\x59\x0c\xc8\x9d\x33\x69\x84\x52\xa8\xf6\x41\x61\x07\xa6\x95\x2f\x12\x6b\xb2\x1e\xe3\xe3\x32\xd2\x28\x5d\xb7\x28\xc0\x9b\xfa\x8c\xbd\x4c\x3b\x13\xb3\x58\xb9\x83\x8d\xea\x7c\xf3\x9d\xc1\x2e\x37\x06\x6a\x09\xcf\x78\x09\xa0\xd0\xea\x06\xc3\xbb\xaa\x14\x77\x64\x00\xf4\x03\xf8\x63\xed\x83\xb5\xbd\xd3\xc2\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00', byteorder='little')
print(hex(n))
c = pow(~key, e, n)
print(c)

