use crate::dersz::*;

use crate::file_ext::*;
use crate::user::User;
use crate::user::UserChild;
use serde::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::path::PathBuf;
use crate::reerr::*;

#[derive(Debug, Clone)]
pub struct Extern {
    pub hash: u32,
    pub path: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TypeDescriptor {
    pub hash: u32,
    pub crc: u32,
}

#[derive(Debug)]
pub struct Rsz {
    pub roots: Vec<u32>,
    pub extern_slots: HashMap<u32, Extern>,
    pub type_descriptors: Vec<TypeDescriptor>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum RszSlot {
    None,
    Extern(Extern),
    Intern(RszValue),
}

impl Rsz {
    pub fn new<F: Read + Seek>(mut file: F, base: u64) -> Result<Rsz> {
        file.seek(SeekFrom::Start(base))?;
        let magic = file.read_magic()?;
        let ext = core::str::from_utf8(&magic)?;
        if ext != "RSZ\0" {
            return Err(Box::new(FileParseError::MagicError { 
                real_magic: String::from("RSZ"), 
                read_magic: ext.to_string()
            }))
        }

        let version = file.read_u32()?;
        if version != 0x10 {
            return Err(format!("Unexpected RSZ version {}", version).into());
        }

        let root_count = file.read_u32()?;
        let type_descriptor_count = file.read_u32()?;
        let extern_count = file.read_u32()?;
        let padding = file.read_u32()?;
        if padding != 0 {
            return Err(format!("Unexpected non-zero padding in RSZ: {}", padding).into());
        }

        let type_descriptor_offset = file.read_u64()?;
        let data_offset = file.read_u64()?;
        let string_table_offset = file.read_u64()?;

        let roots = (0..root_count)
            .map(|_| file.read_u32())
            .collect::<Result<Vec<_>>>()?;

        file.seek_noop(base + type_descriptor_offset)?;

        let type_descriptors = (0..type_descriptor_count)
            .map(|_| {
                let hash = file.read_u32()?;
                let crc = file.read_u32()?;
                Ok(TypeDescriptor { hash, crc })
            })
            .collect::<Result<Vec<_>>>()?;

        if type_descriptors.first() != Some(&TypeDescriptor { hash: 0, crc: 0 }) {
            return Err(format!("The first type descriptor should be 0").into())
        }

        file.seek_assert_align_up(base + string_table_offset, 16)?;

        let extern_slot_info = (0..extern_count)
            .map(|_| {
                let slot = file.read_u32()?;
                let hash = file.read_u32()?;
                let offset = file.read_u64()?;
                Ok((slot, hash, offset))
            })
            .collect::<Result<Vec<_>>>()?;

        let extern_slots = extern_slot_info
            .into_iter()
            .map(|(slot, hash, offset)| {
                file.seek_noop(base + offset)?;
                let path = file.read_u16str()?;
                if !path.ends_with(".user") {
                    return Err(format!("Non-USER slot string").into());
                }
                if hash != type_descriptors
                        .get(usize::try_from(slot)?)
                        .expect("slot out of bound")
                        .hash
                {
                    return Err(format!("slot hash mismatch").into())
                }
                Ok((slot, Extern { hash, path }))
            })
            .collect::<Result<HashMap<u32, Extern>>>()?;
        //println!("{extern_slots:?}");
        //println!("{}", base + data_offset);
        //file.seek(SeekFrom::Start(base + data_offset))?;
        file.seek_assert_align_up(base + data_offset, 16)?;

        let mut data = vec![];
        file.read_to_end(&mut data)?;
        //println!("{:?}", &data[0..128]);
        Ok(Rsz {
            roots,
            extern_slots,
            type_descriptors,
            data,
        })
    }


    pub fn deserializev2(&self, root_dir: Option<String>) -> Result<DeRsz> {
        //println!("{:?}", &self.data[0..128]);
        let mut cursor = Cursor::new(&self.data);
        let mut structs: Vec<RszValue> = Vec::new();
        let mut extern_idxs: HashSet<u32> = HashSet::new();
        for (i, &TypeDescriptor { hash, crc }) in self.type_descriptors.iter().enumerate() {
            if let Some(slot_extern) = self.extern_slots.get(&u32::try_from(i)?) {
                let x = RszType::Extern(slot_extern.path.clone());
                let struct_type = match RszDump::rsz_map().get(&hash) {
                    Some(x) => x,
                    None => return Err(Box::new(FileParseError::InvalidRszTypeHash(hash)))
                };
                let x = struct_type.to_value(x);
                structs.push(x);
                extern_idxs.insert(i as u32);
                /*
                eprintln!("{:?}", slot_extern.path);
                if slot_extern.hash != hash {
                    return Err(format!("Extern hash mismatch").into())
                }
                let real_path = PathBuf::from(root_dir.clone().unwrap_or("/".to_string()));
                let mut real_path = real_path.join("natives/stm").join(&slot_extern.path.to_lowercase());
                real_path.set_extension("user.3");
                eprintln!("{real_path:?}");
                let file = File::open(&real_path)?;
                let extern_rsz = Box::new(User::new(&file)?.rsz);
                let dersz = Box::new(extern_rsz.deserializev2(root_dir.clone())?);
                structs.push(Box::new(dersz.roots[0].clone()));
                //node_buf.push(NodeSlot::Extern(slot_extern.path.clone()));
                //println!("{:?}", node_buf);*/
                continue;
            } else {
                // check for object index and return that too
                let something = RszDump::parse_struct(&mut cursor, TypeDescriptor{hash, crc})?;
                //println!("{something:#?}");
                structs.push(something);
            }
        }

        let mut roots = Vec::new();
        for root in &self.roots {
            println!("ROot: {root}");
            match structs.get(*root as usize) {
                None => eprintln!("Could not find root {}", root),
                Some(obj) => {
                    roots.push(obj.clone());
                    /*match obj {
                        RszSlot::None => todo!(),
                        RszSlot::Extern(_) => todo!(),
                        RszSlot::Intern(rsz_struct) => roots.push(rsz_struct.to_owned()),
                    }*/
                    
                }
            }
        }
        //let results = self.roots.iter().map(|root| {
        //    structs.get(*root as usize).unwrap()
        //}).collect::<Vec<RszStruct<RszType>>>();
        let mut leftover = vec![];
        cursor.read_to_end(&mut leftover)?;
        if !leftover.is_empty() {
            return Err(format!("Left over data {leftover:?}").into());
            //eprintln!("Left over data {leftover:?}");
        }

        Ok(DeRsz {
            roots,
            structs,
            extern_idxs,
        })
    }

    /*pub fn verify_crc(&self, crc_mismatches: &mut BTreeMap<&str, u32>, print_all: bool) {
        for td in &self.type_descriptors {
            if let Some(type_info) = RSZ_TYPE_MAP.get(&td.hash) {
                if print_all
                    || (!type_info.versions.contains_key(&td.crc) && !type_info.versions.is_empty())
                {
                    crc_mismatches.insert(type_info.symbol, td.crc);
                }
            }
        }
    }*/
}


#[derive(Debug, Serialize, Clone)]
#[allow(dead_code)]
pub enum ExternUser<T> {
    Path(String),
    Loaded(T),
}

/*impl<T: FromUser> ExternUser<T> {
    pub fn load<'a>(
        &'a mut self,
        pak: &'_ mut crate::pak::PakReader<impl Read + Seek>,
        version_hint: Option<u32>,
    ) -> Result<&'a mut T> {
        match self {
            ExternUser::Path(path) => {
                let index = pak.find_file(path)?;
                let file = pak.read_file(index)?;
                let user = crate::user::User::new(Cursor::new(file))?;
                *self = ExternUser::Loaded(user.rsz.deserialize_single(version_hint)?);
                if let ExternUser::Loaded(t) = self {
                    Ok(t)
                } else {
                    unreachable!()
                }
            }
            ExternUser::Loaded(t) => Ok(t),
        }
    }

    pub fn unwrap(&self) -> &T {
        match self {
            ExternUser::Path(_) => {
                panic!("ExternUser not loaded")
            }
            ExternUser::Loaded(t) => t,
        }
    }
}*/

/*impl<T> FieldFromRsz for ExternUser<T> {
    fn field_from_rsz(rsz: &mut RszDeserializer) -> Result<Self> {
        rsz.cursor.seek_align_up(4)?;
        let extern_path = rsz.get_extern()?.to_owned();
        Ok(ExternUser::Path(extern_path))
    }
}*/

/*impl<T> FieldFromRsz for Option<ExternUser<T>> {
    fn field_from_rsz(rsz: &mut RszDeserializer) -> Result<Self> {
        rsz.cursor.seek_align_up(4)?;
        let extern_path = rsz.get_extern_opt()?;
        Ok(extern_path.map(|p| ExternUser::Path(p.to_owned())))
    }
}*/
