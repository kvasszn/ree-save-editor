use std::{ffi::{c_void, CStr}, path::Path, ptr};

#[cfg(target_os="linux")]
use libc::{syscall, SYS_arch_prctl, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE};
#[cfg(target_os="windows")]
use windows_sys::Win32::{System::Memory, Foundation::{HANDLE}};


use crate::reerr::Result;

#[repr(C)]
struct FakeTEB {
    tls_slots: [*mut u8; 64],
    reserved: [u64; 4],
}

#[cfg(target_os="linux")]
#[unsafe(no_mangle)]
pub extern "win64" fn HeapAlloc(_heap: *mut c_void, _flags: u32, size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    let res = unsafe { libc::malloc(size) };
    println!("Hooked Alloc {:?}, {size}", res);
    res
}

#[cfg(target_os="linux")]
#[unsafe(no_mangle)]
pub extern "win64" fn HeapFree(_heap: *mut c_void, _flags: u32, ptr: *mut c_void) -> i32 {
    println!("Hooked Free {ptr:?}");
    if !ptr.is_null() {
        unsafe { libc::free(ptr) }
    }
    return 1;
}

#[cfg(target_os="windows")]
#[unsafe(no_mangle)]
pub extern "win64" fn HeapAlloc(_heap: *mut c_void, _flags: u32, size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    unsafe {
        let res = Memory::HeapAlloc(HANDLE(_heap), Memory::HEAP_FLAGS(_flags), size);
        println!("Hooked Alloc {:?}, {size}", res);
        res
    }
}

#[cfg(target_os="windows")]
#[unsafe(no_mangle)]
pub extern "win64" fn HeapFree(_heap: *mut c_void, _flags: u32, ptr: *mut c_void) -> i32 {
    println!("Hooked Free {ptr:?}");
    if !ptr.is_null() {
        let res = Memory::HeapFree(Memory::GetProcessHeap().unwrap(), Memory::HEAP_FLAGS(_flags), Some(ptr));
    }
    return 1;
}

#[unsafe(no_mangle)]
pub extern "win64" fn DefaultHook() {
    println!("Unknown Hook");
}

pub fn read_usize (x: &[u8], offset: usize) -> usize {
    usize::from_le(unsafe {
        ptr::read_unaligned(x[offset..].as_ptr() as *const usize)
    })
}

pub fn read_u32 (x: &[u8], offset: usize) -> u32 {
    u32::from_le(unsafe {
        ptr::read_unaligned(x[offset..].as_ptr() as *const u32)
    })
}

#[cfg(target_os="windows")]
fn alloc_program(size: usize) -> *mut c_void {
    unsafe {
        VirtualAlloc(std::ptr::null_mut(), size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE)
    }
}

#[cfg(target_os="windows")]
fn dealloc(ptr: *mut c_void, size: usize) -> i32 {
    unsafe {
        VirtualFree(ptr, size, MEM_RELEASE) as i32
    }
}

#[cfg(target_os="linux")]
fn alloc_program(size: usize) -> *mut c_void {
    unsafe {
        let ptr = libc::mmap(ptr::null_mut(), size,
            PROT_READ | PROT_WRITE | PROT_EXEC,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1, 0
        );
        if ptr == MAP_FAILED {
            panic!("Failed to map decrypt binary to memory");
        }
        ptr
    }
}


#[cfg(target_os="linux")]
fn dealloc_program(ptr: *mut c_void, size: usize) -> i32 {
    unsafe {
        libc::munmap(ptr, size) as i32
    }
}

static DECRYPT_PROGRAM: &[u8] = include_bytes!("../rev/decrypt.bin");
type DecryptFn = extern "win64" fn(*mut u8, *const u8, usize, u64) -> i32;
type EncryptFn = extern "win64" fn(*mut u8, *const u8, usize, u64, *mut usize) -> i32;
type SetPFn= extern "win64" fn(u64) -> i32;

#[allow(unused)]
pub struct Mandarin {
    teb: Box<FakeTEB>,
    program: *mut c_void,
    size: usize,
    decrypt_fn: DecryptFn,
    encrypt_fn: EncryptFn,
    set_p_fn: SetPFn,
}

impl Mandarin {
    pub const NAMES_OFFSET_OFFSET: usize = 0x28018;
    pub const FUNCTIONS_OFFSET_OFFSET: usize = 0x28028;
    pub const DECRYPT_OFFSET: usize = 0x4c80;
    pub const ENCRYPT_OFFSET: usize = 0x3b40;
    pub const SET_P_OFFSET: usize = 0x3b30;
    pub const ARCH_SET_GS: u64 = 0x1001;
    pub fn init() -> Mandarin {

        let teb = Box::new(FakeTEB {
            tls_slots: [std::ptr::null_mut(); 64],
            reserved: [0; 4],
        });

        let teb_ptr = &*teb as *const _ as u64;
        unsafe {
            syscall(SYS_arch_prctl, Self::ARCH_SET_GS, teb_ptr);
        }

        let program = &mut DECRYPT_PROGRAM.to_vec();
        let name_offsets = read_usize(program, Self::NAMES_OFFSET_OFFSET);
        let _names_end = read_u32(program, Self::NAMES_OFFSET_OFFSET + 0x4 + 0x8);
        let function_offsets = read_usize(program, Self::FUNCTIONS_OFFSET_OFFSET);
        let mut count = 0;
        while name_offsets + count * 0x8 + 0x8 < program.len() {
            let name_addr = read_usize(program, name_offsets + count * 0x8);
            if name_addr == 0x0 {
                break;
            }
            let function_ptr = function_offsets + count * 0x8;
            //let function_value = read_usize(program, function_ptr);
            let name = unsafe {CStr::from_ptr(program[name_addr + 0x2..].as_ptr() as *const i8)}
            .to_str().unwrap();

            let new_func_addr = match name {
                "HeapAlloc" => HeapAlloc as usize,
                "HeapFree" => HeapFree as usize,
                _ => DefaultHook as usize
            };

            program[function_ptr..function_ptr + 0x8].copy_from_slice(&new_func_addr.to_le_bytes());
            //println!("hooked {name}@{function_ptr:08x}: old={function_value:08x} new={new_func_addr:08x}");
            count += 1;
        }
        //program[0x2ace0..0x2ace8].copy_from_slice(&0x0u64.to_le_bytes());

        let program_ptr = alloc_program(program.len());
        unsafe {
            let program_slice: &mut [u8] = std::slice::from_raw_parts_mut(program_ptr as *mut u8, program.len());
            program_slice.copy_from_slice(program);

            let decrypt_fn: DecryptFn = std::mem::transmute(program_ptr.wrapping_add(Self::DECRYPT_OFFSET));
            let encrypt_fn: EncryptFn = std::mem::transmute(program_ptr.wrapping_add(Self::ENCRYPT_OFFSET));
            let set_p_fn: SetPFn = std::mem::transmute(program_ptr.wrapping_add(Self::SET_P_OFFSET));
            Mandarin {
                teb,
                program: program_ptr,
                size: program.len(),
                decrypt_fn,
                encrypt_fn,
                set_p_fn,
            }
        }
    }

    pub fn uninit(&self) {
        dealloc_program(self.program, self.size);
    }

    pub fn decrypt_file(&self, save_file: &Path, key: u64) -> Result<Vec<u8>> {
        let encrypted = std::fs::read(save_file).unwrap();
        let decrypted_len = read_usize(&encrypted, encrypted.len() - 12);
        self.decrypt_bytes(&encrypted[0x10..encrypted.len() - 12], decrypted_len, key)
    }

    pub fn decrypt_bytes(&self, encrypted: &[u8], decrypted_len: usize, key: u64) -> Result<Vec<u8>> {
        println!("DECRYPTING with key=0x{key:016x}, decrypted length=0x{decrypted_len:08x}");
        let mut decrypted = vec![0; decrypted_len];
        let result = (self.decrypt_fn)(decrypted.as_mut_ptr(), encrypted.as_ptr(), decrypted_len, key);
        if result == 1 {
            println!("[Successfully Decrypted]");
            return Ok(decrypted);
        } else {
            println!("[Error Decrypting]");
            return Err(format!("Failed to decrypt").into());
        }
    }

    pub fn encrypt_bytes(&self, decrypted: &[u8], key: u64) -> Result<Vec<u8>> {
        println!("ENCRYPTING with key=0x{key:016x}, decrypted length=0x{:08x}", decrypted.len());
        let mut encrypted = vec![0; 0x100000]; // idk just fucking allocate enough space for the
                                               // encrypted buffer
                                               // im pretty sure the game uses the preallocated
                                               // decrypted + decompressed buffer for this,
                                               // so it has enough room
        let len_ptr: *mut usize = std::ptr::null_mut(); // the length gets returned here which is nice ig
        let result = (self.encrypt_fn)(encrypted.as_mut_ptr(), decrypted.as_ptr(), decrypted.len(), key, len_ptr);
        let len = unsafe { *len_ptr };
        // if i use len here, it has the wrong value, i think this is a bug in rust
        // or i have undefined behaviour but i dont know it
        if result != 0x0 {
            println!("[Successfully encrypted] decrypted length=0x{:016x}", len);

            return Ok(encrypted);
        } else {
            println!("[Error encrypting]");
            return Err(format!("Failed to encrypt").into());
        }
    }

    pub fn sanity_check(save_file: &Path) {
        println!("Running sanity check");
        let mandarin = Mandarin::init();
        let key = 0x01100001_1168AFC6;
        //let key: u64 = 0x01100001_00000000;

        //for v in 0..u32::MAX {
        //    let decrypted = mandarin.decrypt_file(save_file, v as u64 + key);
        //    if decrypted.is_ok() {
        //        panic!("{v}");
       //     }
        //}

        let decrypted = mandarin.decrypt_file(save_file, key).unwrap();
        std::fs::write("decrypted.bin", &decrypted).unwrap();
        let encrypted = mandarin.encrypt_bytes(&decrypted, key).unwrap();
        let og = std::fs::read(save_file).unwrap();
        //println!("original={:?}", &og[0x10..]);
        //println!("re-encrypted={:?}", &encrypted[0..og.len()-12-0x10]);
        std::fs::write("re-encrypted.bin", &encrypted).unwrap();
        for _i in 0..og.len() - 0x10 {
            //if og[0x10 + i] != encrypted[i] {
            //println!("Bytes not equal at {i:016x}");

            //}
        }
        //let decrypted = mandarin.decrypt_bytes(&encrypted[0..], 0x6493, key).unwrap();
        //let encrypted = mandarin.encrypt_bytes(&decrypted, key).unwrap();
        //let decrypted = mandarin.decrypt_bytes(&encrypted[0..], 0x6493, key).unwrap();

    }
}
