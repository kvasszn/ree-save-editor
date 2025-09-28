#include <cstdint>
#include <cstdio>
#ifdef __linux__
    #include <sys/mman.h>
    #include <asm/prctl.h>
    #include <sys/prctl.h>
#else
    #include <windows.h>
#endif
#include <cstring>
#include <fstream>
#include <vector>
#include <iostream>
#include <algorithm>


#include <cstdlib>
#include <cstdint>

#ifdef __linux__
#define MSABI __attribute__((ms_abi))
typedef struct FakeTEB {
    void* tls_slots[64];
    uint64_t reserved[4]; // match offsets your binary accesses
} FakeTEB;

FakeTEB teb = {0};  // zero-initialized like Windows

using HANDLE = void*;
using SIZE_T = std::size_t;
using ULONG  = unsigned long;

// Export C ABI, use MS calling convention
extern "C" {

    __attribute__((ms_abi))
    void* HeapAlloc(HANDLE heap, ULONG flags, SIZE_T size) {
        (void)heap;  // ignore heap handle
        (void)flags; // ignore flags
        printf("alloc size: %lx", size);
        return malloc(size);
    }

    __attribute__((ms_abi))
    int HeapFree(HANDLE heap, ULONG flags, void* mem) {
        (void)heap;
        (void)flags;
        printf("free: %p", mem);
        free(mem);
        return 1;
    }

    __attribute__((ms_abi))
    HANDLE GetProcessHeap() {
        return reinterpret_cast<HANDLE>(0x1);
    }

}
#endif


struct FunctionName {
    uint16_t idk;
    char first_char;
};

typedef struct {
    uint64_t names_offset;
    uint32_t idk;
    uint32_t dll_name_offset;
    uint64_t functions_offset;
} LibraryFunctionOffsets;

using DecryptFunc = int(MSABI*)(uint8_t* dst, uint8_t* src, uint64_t len, uint64_t key);

uint64_t key = 0x011000011168AFC6;
uint64_t func_desc_offset = 0x28018;
uint64_t heap_handle_offset = 0x2ac68;

int filter = 0;
std::vector<std::string> required_functions = { "HeapAlloc", "HeapFree" };



uint8_t* patch_code(uint8_t* code, uint64_t size) {
#ifdef WIN32
    HMODULE hModule = LoadLibraryA("kernel32.dll"); // or any DLL
    if (!hModule) {
        std::cerr << "Failed to load DLL\n";
        return nullptr;
    }
#endif
    LibraryFunctionOffsets* kernel32dll_offsets = (LibraryFunctionOffsets*)(code + func_desc_offset);
    uint64_t* name_offsets = (uint64_t*)(code + kernel32dll_offsets->names_offset);
    uint64_t* function_ptrs = (uint64_t*)(code + kernel32dll_offsets->functions_offset);
    for (uint64_t i = 0; i < 68; i++) {
        char* func_name = (char*)(name_offsets[i] + code + 2);
        auto it = std::find(required_functions.begin(), required_functions.end(), std::string(func_name));
        if (it == required_functions.end()) {
            printf("Skipped %s\n", func_name);
            //continue;
        };
        uint64_t func_new_addr = 0x0;
        if (std::strcmp("HeapAlloc", func_name) == 0) {
            func_new_addr = (uint64_t)&HeapAlloc;
        }
        else if (std::strcmp("HeapFree", func_name) == 0) {
            func_new_addr = (uint64_t)&HeapFree;
        }
#ifdef WIN32
        else {
            func_new_addr = (uint64_t)GetProcAddress(hModule, func_name);
            if (!func_new_addr) {
                std::cerr << "Function not found" << func_name <<"\n";
                continue;
            }
        }
#endif
        uint64_t old_addr = (uint64_t)function_ptrs[i]; 
        function_ptrs[i] = (uint64_t)func_new_addr;
        printf("Replaced %s: old=%lx, new=%lx, addr=%p\n", func_name, old_addr, function_ptrs[i], &function_ptrs[i]);
    }
    uint64_t* heapHandleSlot = (uint64_t*)(code + heap_handle_offset);
    *heapHandleSlot = (uint64_t)GetProcessHeap();
    return code;
}

int decrypt_save(char* code_file, char* save_file_path) {
    std::ifstream f(code_file, std::ios::binary);
    if (!f) {
        std::cerr << "Cannot open file\n";
        return 1;
    }
    std::vector<uint8_t> code((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
    uint8_t* patched_code = patch_code(code.data(), code.size());
#ifdef __linux__
    uint8_t* exec_mem = (uint8_t*)mmap(
        nullptr,
        code.size(),
        PROT_READ | PROT_WRITE, // allow RWX
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0
    );
    if (exec_mem == MAP_FAILED) {
        printf("Map failed");
    }
#else
    void* exec_mem = VirtualAlloc(nullptr, code.size(), MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
#endif

    if (!exec_mem) {
        std::cerr << "VirtualAlloc/Mmap failed\n";
        return 1;
    }

    printf("exec_mem allocated at %p\n", exec_mem);
    printf("patched_code at %p\n", (uint8_t*)code.data());
    std::memcpy((uint8_t*)exec_mem, code.data(), code.size());
    printf("Prepared exec mem\n");
    mprotect((uint8_t*)exec_mem, code.size(), PROT_READ | PROT_WRITE | PROT_EXEC);
    DecryptFunc decrypt = (DecryptFunc)((unsigned char*)exec_mem + 0x3153a);


    std::ifstream save_file(save_file_path, std::ios::binary);
    std::vector<unsigned char> save_data((std::istreambuf_iterator<char>(save_file)), std::istreambuf_iterator<char>());

    printf("Prepared Buffers\n");
    uint8_t* src = reinterpret_cast<uint8_t*>(save_data.data());
    uint64_t len;

    std::memcpy(&len, src + save_data.size() - 12, sizeof(len));
    uint8_t* dst = (uint8_t*)malloc(sizeof(uint8_t) * len);

    printf("decrypted length=0x%lx\n", len);
    printf("using key: 0x%08lx\n", key);

    int result = decrypt(dst, src + 0x10, len, key);
    if (result == 1) {
        printf("Successfully decrypted\n");
    } else {
        printf("Error decrypting\n");
        return 1;
    }

    std::ofstream file("output.bin", std::ios::binary);
    if (!file) {
        return 1; // failed to open
    }
    f.close();
    file.write(reinterpret_cast<const char*>(dst), len);
    file.close();
    save_file.close();
    free(dst);
#ifdef WIN32
    VirtualFree(exec_mem, 0, MEM_RELEASE);
#else
    munmap(exec_mem, code.size());
#endif
    return 0;
}

int main(int argc, char** argv) {
#ifdef __linux__
    asm volatile("mov %0, %%rax; wrgsbase %%rax" : : "r"(&teb) : "rax");
#endif
    if (argc < 3) {
        printf("Invalid args\n");
        printf("arg 1 = binary\n");
        printf("arg 2 = save file\n");
    }
    //for (filter = 39; filter < 68; filter++) {
        decrypt_save(argv[1], argv[2]);
    //}
    return 0;
}
