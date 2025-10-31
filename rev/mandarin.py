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
