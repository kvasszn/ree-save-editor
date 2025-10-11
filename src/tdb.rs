use std::{
    fs::File,
    io::{Cursor, Read}
};
use bitfield::bitfield;
use file_macros::StructRW;

use crate::{reerr::{self, FileParseError, Result}, file::StructRW};
#[repr(C)]
#[derive(Debug, file_macros::StructRW)]
pub struct TDBHeader {
    #[magic = b"TDB\0"]
    magic: [u8; 4],
    version: u32,
    num_types: u32,
    types_start_of_generics_probably: u32,
    unk_int_tdb74: u32,
    num_methods: u32,
    num_fields: u32,
    num_types_impl: u32,
    num_field_impl: u32,
    num_method_impl: u32,
    num_property_impl: u32,
    num_properties: u32,
    num_events: u32,
    num_params: u32,
    num_attributes: u32,
    num_init_data: i32,
    num_attributes2: u32,
    num_intern_strings: u32,
    num_modules: u32,
    dev_entry: i32,
    app_entry: i32,
    num_string_pool: u32,
    num_byte_pool: u32,
    padding: u32,
    modules: u64,
    types: u64,
    types_impl: u64,
    methods: u64,
    methods_impl: u64,
    fields: u64,
    fields_impl: u64,
    properties: u64,
    properties_impl: u64,
    events: u64,
    params: u64,
    attributes: u64,
    init_data: u64,
    unk: u64,
    attributes2: u64,
    string_pool: u64,
    byte_pool: u64,
    intern_strings: u64
}

bitfield!{
    pub struct RETypeImplBitfield(u64);
    impl Debug;
    unk_pad, _: 32, 0;
    pub num_member_fields, _: 47, 33;
    unk_pad2, _: 54, 48;
}

#[repr(C)]
#[derive(Debug)]
pub struct RETypeImpl {
    name_offset: i32,
    namespace_offset: i32,
    field_size: i32,
    static_field_size: i32,
    b1: RETypeImplBitfield,
    num_member_methods: u16,
    num_native_vtable: i16,
    interface_id: i16,
    unk: u16,
    unk2: u32,
    unk1: u32,
    unk4: u64,
    //pad_1e: [u8; 0x12],
}


bitfield!{
    pub struct RETypeDefB1(u64);
    impl Debug;
    pub index, _: 18, 0;
    pub parent_typeid, _: 37, 19;
    pub declaring_typeid, _: 56, 38;
    pub underlying_typeid, _: 63, 57;
}

bitfield!{
    pub struct RETypeDefB2(u64);
    impl Debug;
    pub array_typeid_tbd, _: 18, 0;
    pub element_typeid_tbd, _: 37, 19;
    pub impl_index, _: 55, 38;
    pub system_typeid, _: 62, 56;
}

bitfield!{
    pub struct RETypeDefB3(u64);
    impl Debug;
    pub default_ctor, _: 21, 0;
    pub member_method, _: 43, 22;
    pub member_field, _: 63, 44;
}

bitfield!{
    pub struct RETypeDefB4(u64);
    impl Debug;
    u32;
    pub num_member_prop, _: 11, 0;
    pub member_prop, _: 30, 12;
    pub unk_data, _: 57, 32;
    pub object_type, _: 60, 58;
}

bitfield!{
    pub struct RETypeDefB5(u64);
    impl Debug;
    pub unk_data_before_generics, _: 25, 0;
    pub generics, _: 51, 26;
    pub interfaces, _: 63, 52;
}


#[repr(C)]
#[derive(Debug)]
pub struct RETypeDefinition {
    b1: RETypeDefB1,
    b2: RETypeDefB2,
    type_flags: u32,
    size: u32,
    fqn_hash: u32,
    type_crc: u32,
    b3: RETypeDefB3,
    b4: RETypeDefB4,
    b5: RETypeDefB5,
    r#type: u64, // RETypeCLR*
    managed_vt: u64,
    unk_new_tdb74_uint64: u64,
}

pub fn pattern_search(buf: &[u8], pattern: &[u8]) -> usize {
    let mut matched = 0;
    let mut idx = 0;
    while idx < buf.len() {
        if buf[idx] == pattern[matched] {
            matched += 1;
            if matched == pattern.len() {
                idx -= pattern.len() - 1;
                break;
            }
            idx += 1;
        } else if matched != 0 {
            idx += matched;
            matched = 0;
        } else {
            matched = 0;
            idx += 1;
        }
    }
    idx
}

use std::ffi::{CStr};
use std::os::raw::c_char;

unsafe fn c_str_to_str<'a>(ptr: *const c_char) -> &'a str {
    // Safety: caller must guarantee `ptr` is valid C string
    unsafe {
        let c_str = CStr::from_ptr(ptr);
        c_str.to_str().expect("invalid UTF-8")
    }
}

impl TDBHeader {
    pub fn from_exe(file: &mut File) -> Result<Self> {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let pattern = "TDB\0".as_bytes();
        let idx = pattern_search(&buf, pattern);
        println!("found at {idx:?}");
        let mut reader = Cursor::new(&buf);
        let tdb_header = TDBHeader::read(&mut reader, &mut ())?;

        println!("{:08X}", tdb_header.types_impl - 0x140000000);
        println!("str_pool {:08X}", tdb_header.string_pool - 0x140000000);
        println!("{tdb_header:?}");


        let t = tdb_header.types_impl as usize - 0x140000000;
        let ptr = buf[t..].as_ptr() as *const RETypeImpl;
        let types_impl = unsafe { std::slice::from_raw_parts(ptr, tdb_header.num_types_impl as usize) };

        for t in &types_impl[0..10] {
            println!("{:?}: {:?}", tdb_header.get_string(t.name_offset as u32, buf.as_ptr()), t);
        }

        let t = tdb_header.types as usize - 0x140000000;
        let ptr = buf[t..].as_ptr() as *const RETypeDefinition;
        let types = unsafe { std::slice::from_raw_parts(ptr, tdb_header.num_types as usize) };
        for t in &types[0..] {
            if t.b1.index() == 138750 {

                let impl_index = t.b2.impl_index() as usize;
                let type_impl = &types_impl[impl_index];
                println!("{:08x}, {:08x}, {:08x}, {:08x}", t.fqn_hash, t.type_crc, type_impl.name_offset, 0);
                println!("{:?}\n", type_impl);
                println!("{:?}: {:?}", tdb_header.get_string(type_impl.name_offset as u32, buf.as_ptr()), t);
                println!("");
            }
        }


        println!("str_pool {:08X}", tdb_header.string_pool - 0x140000000);
        Ok(tdb_header)
    }

    fn string_pool_bitmask(&self) -> u64 {
        let mut res = 1;
        while res < self.num_string_pool as u64 {
            res <<= 1;
        }
        res - 1
    }

    fn get_string<'a>(&'a self, offset: u32, buf: *const u8) -> Option<&'a str> {
        let bitmask = self.string_pool_bitmask();
        let offset = offset as u64 & bitmask;
        if offset > self.num_string_pool as u64 {
            None
        } else {
            unsafe {
                let string_pool_ptr = buf.add(self.string_pool as usize - 0x140000000) as *const u8;
                let name_ptr = string_pool_ptr.add(offset as usize) as *const c_char;
                println!("{:08x}", self.string_pool - 0x140000000 + offset as u64);
                Some(c_str_to_str(name_ptr))
            }
        }
    }
}
