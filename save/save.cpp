#include <cstdint>
#include <sys/mman.h>
#include <cstring>
#include <fstream>
#include <vector>
#include <iostream>

int main() {
    std::ifstream f("/mnt/windows/Users/nikola/mh/monsterhunterwilds_000000000A990000_virtualized_decryption.bin", std::ios::binary);
    if (!f) {
        std::cerr << "Cannot open file\n";
        return 1;
    }

    using DecryptFunction = int(*)(uint8_t* dst, uint8_t* src, uint64_t len, uint64_t key);
    std::vector<unsigned char> code((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
    void* exec_mem = mmap(nullptr, code.size(),
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);

    if (exec_mem == MAP_FAILED) {
        perror("mmap");
        return 1;
    }

    // Copy binary into executable memory
    std::memcpy(exec_mem, code.data(), code.size());

    std::ifstream save_file("/home/nikola/.local/share/Steam/userdata/292073414/2246340/remote/win64_save/data00-1.bin", std::ios::binary);
    std::vector<unsigned char> save_data((std::istreambuf_iterator<char>(save_file)), std::istreambuf_iterator<char>());
    DecryptFunction decrypt = (DecryptFunction)((unsigned char*)exec_mem + 0x3153a);
    uint8_t* dst = (uint8_t*)malloc(sizeof(uint8_t) * 0x50000);
    uint8_t* src = reinterpret_cast<uint8_t*>(save_data.data());
    uint64_t len;
    std::memcpy(&len, src + save_data.size() - 12, sizeof(len));
    uint64_t key = 0x011000011168AFC6;
    printf("%lx\n", len);
    void* func = (unsigned char*)exec_mem + 0x3153a;
    int result;
    asm volatile(
        "sub $32, %%rsp\n"       // shadow space
        "mov %1, %%rcx\n"        // arg1 -> RCX
        "mov %2, %%rdx\n"        // arg2 -> RDX
        "mov %3, %%r8\n"         // arg3 -> R8
        "mov %4, %%r9\n"         // arg4 -> R9
        "call *%5\n"
        "add $32, %%rsp\n"
        "mov %%eax, %0\n"        // result in output
        : "=r"(result)
        : "r"(dst), "r"(src + 0x10), "r"(len), "r"(key), "r"(func)
        : "rcx", "rdx", "r8", "r9", "rax", "memory"
    );

    std::cout << "Result: " << result << "\n";
    //int result = decrypt(dst, src, len, key);
    free(dst);
}
