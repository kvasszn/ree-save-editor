use std::{error::Error, io::{Read, Seek}};

use indexmap::IndexMap;
use util::{ReadExt, SeekExt, seek_align_up};

use crate::{save::{SaveFile, game::{Game}, types::{Array, ArrayType, Class, Field, FieldType, FieldValue, Struct}}, sdk::{StringU16, type_map::{FieldInfo, TypeInfo, TypeMap, murmur3}}};

// There are two types of corruption that I think can be fixed
// The first is missing data/incorrect headers, where the size of the data has not changed
// This can be fixed by comparing to what should be there in a normal save, or based on type
// information from RSZ
// The second is just scanning for class type hashes, technically this could be done with the type
// enums however there would probably be alot of failures for that, so I think just keeping it to
// "objects/classes" makes more sense 
//
// Actually applying the fix would probably involve copying "good" objects to a default save file
// In the case of the first method, it might be possible to not have to copy anything
//
// Also, It's important to note that saves corrupted at the deflate or encryption level are
// probably unrecoverable (definitely for encryption, maybe possible for deflate?)
pub struct CorruptSaveReader<'a> {
    type_map: &'a TypeMap,
    game: Game,
}

impl<'a> CorruptSaveReader<'a> {
    pub fn new(type_map: &'a TypeMap, game: Game) -> Self {
        Self {
            type_map,
            game
        }
    }

    pub fn read_array<R: Read + Seek>(&mut self, reader: &mut R, field_info: &FieldInfo) -> Result<Array, Box<dyn Error>> {
        reader.seek_align_up(4)?;
        let member_type = FieldType::try_from(reader.read_i32()?)?;
        let member_size = reader.read_u32()?;
        let len = reader.read_u32()?;
        let array_type = ArrayType::try_from(reader.read_i32()?)?;
        let mut values: Vec<FieldValue> = Vec::with_capacity(len as usize);
        let mut broken = false;
        let type_info = field_info.get_original_type_array(self.type_map).unwrap();
        for _i in 0..len {
            let value = match array_type {
                ArrayType::Value => {
                    if member_type == FieldType::String { 
                        reader.seek_align_up(4)?;
                        let size = reader.read_u32()?;
                        let data = (0..size).map(|_| {
                            Ok(reader.read_u16()?)
                        }).collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
                        FieldValue::String(Box::new(StringU16::new(data)))
                    } else {
                        self.read_field_value_sized(reader, member_type, member_size, field_info)?
                    }
                },
                ArrayType::Class => {
                    let class = self.read_class(reader, type_info);
                    if class.is_err() {
                        eprintln!("[ERROR] in array class element {class:?}");
                        broken = true;
                        let last_value = values[0].clone();
                        last_value
                    } else {
                        let class = class.unwrap();
                        FieldValue::Class(class.into())
                    }
                },
            };

            if broken { 
                //eprintln!("[WARNING] Replacing Array Elements");
                for _ in _i..len {
                    //values.push(value.clone());
                }
                break;
            } else {
                values.push(value);
            }
        }
        reader.seek_align_up(4)?;
        Ok(Array {
            member_type,
            member_size,
            array_type,
            values
        })
    }

    pub fn read_field_value<R: Read + Seek>(&mut self, reader: &mut R, field_type: FieldType, field_info: &FieldInfo) -> Result<FieldValue, Box<dyn Error>> {
        let value = match field_type {
            FieldType::Unknown => { 
                /*println!("Unknown Field Type found");*/ 
                return Err("Unknown Field Type".into())
            }
            FieldType::Array => { 
                FieldValue::Array(self.read_array(reader, field_info)?.into())
            }
            FieldType::Class => { 
                // TODO: remove unwrap
                let type_info = field_info.get_original_type(self.type_map).unwrap();
                FieldValue::Class(self.read_class(reader, type_info)?.into())
            }
            FieldType::String => { 
                reader.seek_align_up(4)?;
                let size = reader.read_u32()?;
                let data = (0..size).map(|_| {
                    Ok(reader.read_u16()?)
                }).collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
                FieldValue::String(Box::new(StringU16::new(data)))
            }
            // TODO: Add Struct weird shit handling
            // These values actually need a size/len
            _ => {
                reader.seek_align_up(4)?;
                let size = reader.read_u32()?;
                self.read_field_value_sized(reader, field_type, size, field_info)?
            }
        };
        Ok(value)
    }

    // When this is run for the Array struct, it should never be able to read an Object, I'm not
    // sure about array though
    pub fn read_field_value_sized<R: Read + Seek>(&mut self, reader: &mut R, field_type: FieldType, size: u32, field_info: &FieldInfo) -> Result<FieldValue, Box<dyn Error>> {
        if field_type != FieldType::String {
            reader.seek_align_up(size as u64)?;
        }
        let value = match field_type {
            FieldType::Enum => { FieldValue::Enum(reader.read_i32()?) }
            FieldType::Boolean => { FieldValue::Boolean(reader.read_bool()?) }
            FieldType::S8 => { FieldValue::S8(reader.read_i8()?) }
            FieldType::U8 => { FieldValue::U8(reader.read_u8()?) }
            FieldType::S16 => { FieldValue::S16(reader.read_i16()?) }
            FieldType::U16 => { FieldValue::U16(reader.read_u16()?) }
            FieldType::S32 => { FieldValue::S32(reader.read_i32()?) }
            FieldType::U32 => { FieldValue::U32(reader.read_u32()?) }
            FieldType::S64 => { FieldValue::S64(reader.read_i64()?) }
            FieldType::U64 => { FieldValue::U64(reader.read_u64()?) }
            FieldType::F32 => { FieldValue::F32(reader.read_f32()?) }
            FieldType::F64 => { FieldValue::F64(reader.read_f64()?) }
            FieldType::C8 => { FieldValue::C8(reader.read_u8()?) }
            FieldType::C16 => { FieldValue::C16(reader.read_u16()?) }
            FieldType::Array => { FieldValue::Array(self.read_array(reader, field_info)?.into())}
            FieldType::Struct => { 
                let mut data = vec![0u8; size as usize];
                reader.read_exact(&mut data)?;
                FieldValue::Struct(Box::new(Struct{ data }))
            }
            _ => return Err(format!("Unexpected sized read of {:?} for {:?}", size, field_type).into())
        };
        return Ok(value)
    }

    pub fn read_field<R: Read + Seek>(&mut self, reader: &mut R, parent: &TypeInfo, field: &FieldInfo) -> crate::file::Result<Field> {
        let _hash = reader.read_u32()?;
        let field_type_raw = reader.read_i32()?;
        let mut field_type = FieldType::try_from(field_type_raw)?;
        let good_field_type = FieldType::from_field_info(field);
        if field_type != good_field_type {
            eprintln!("[WARNING] Field type mismatch good={good_field_type:?}, read={field_type_raw:?} in field {} from class {}", field.name, parent.name);
            field_type = good_field_type;
        }
        let value = self.read_field_value(reader, field_type, field)?;
        seek_align_up(reader, 4)?;
        Ok(Field {
            hash: field.hash,
            field_type,
            value
        })
    }

    // this breaks on polymorphism
    pub fn read_class<R: Read + Seek>(&mut self, reader: &mut R, type_info: &TypeInfo) -> crate::file::Result<Class> {
        let num_fields = match reader.read_u32() {
            Err(e) => {
                eprintln!("[WARNING] Could not read num fields in class {}: {e}", type_info.name);
                return Ok(Class { num_fields: 0, fields: IndexMap::new(), hash: 0})
            }
            Ok(r) => r
        };
        let mut hash = match reader.read_u32() {
            Err(e) => {
                eprintln!("[WARNING] Could not read hash in class {}: {e}", type_info.name);
                return Ok(Class { num_fields, fields: IndexMap::new(), hash: 0})
            }
            Ok(r) => r
        };
        let correct_hash = murmur3(&type_info.name, 0xffffffff);
        if hash != correct_hash {
            hash = correct_hash;
        }

        let num_fields = if num_fields == 0 {
            type_info.fields.len() as u32
        } else {
            num_fields.max(type_info.fields.len() as u32)
        };

        let mut fields = IndexMap::<u32, Field>::new();
        // could maybe do something where you keep on trying to read a field hash until you cant
        // could also maybe just read in order of the RSZ, if its there, its there, otherwise just
        // ignore
        for (field_hash, field_info) in &type_info.fields {
            let read_field_hash = match reader.read_u32() {
                Err(e) => {
                    eprintln!("[WARNING] Could not read field hash in class {} for field {}: {e}", type_info.name, field_info.name);
                    return Ok(Class {
                        num_fields,
                        hash,
                        fields
                    })
                }
                Ok(r) => r
            };
            let _ = reader.seek_relative(-4);
            if *field_hash != read_field_hash && read_field_hash != 0 {
                //continue;
            }

            let field = self.read_field(reader, type_info, field_info);
            match field {
                Ok(field) => {fields.insert(field.hash, field);},
                Err(e) => {
                    eprintln!("[ERROR] Parsing error on field {}, {e}", field_info.name);
                    return Ok(Class {
                        num_fields,
                        hash,
                        fields
                    })
                }
            }
        }

        /* for _ in 0..num_fields {
           let field_hash = reader.read_u32()?;
        // if the field hash is correct, read normally
        if let Some(field_info) = type_info.fields.get(&field_hash) {
        let field = self.read_field(reader, type_info, field_info);
        match field {
        Ok(field) => {fields.insert(field.hash, field);},
        Err(e) => {
        eprintln!("[ERROR] Parsing error {e}");
        return Ok(Class {
        num_fields,
        hash,
        fields
        })
        }
        }
        } else {
        break;
        }
        }*/
        // Safely read up to a max of type_info 
        Ok(Class {
            num_fields,
            hash,
            fields
        })
    }

    pub fn read_missing_and_scan<R: Read + Seek>(&mut self, reader: &mut R) -> SaveFile {
        // First get the top level struct for each game/file
        // TODO: rn just for wilds, add based on game later
        let type_info = self.type_map.get_by_name("app.savedata.cUserSaveData").unwrap();
        // need to add custom class readers that also have type information along side them when
        let mut fields = Vec::new();
        if let Ok(native_field_hash) = reader.read_u32() {
            let class = self.read_class(reader, type_info).unwrap();
            if native_field_hash == 0 {
                fields.push((murmur3(&"app.savedata.cUserSaveData", 0xffffffff), class));
            } else {
                fields.push((native_field_hash, class));
            }
        }
        let type_info = self.type_map.get_by_name("via.storage.saveService.SaveFileDetail").unwrap();
        if let Ok(native_field_hash) = reader.read_u32() {
            let class = self.read_class(reader, type_info).unwrap();
            if native_field_hash == 0 {
                fields.push((murmur3(&"via.storage.saveService.SaveFileDetail", 0xffffffff), class));
            } else {
                fields.push((native_field_hash, class));
            }
        }
        reader.seek(std::io::SeekFrom::Start(0)).unwrap();
        let scanned_classes = self.scan_classes(reader);
        let mut scanned_classes: Vec<(u32, Class)> = scanned_classes.into_iter().enumerate().map(|(i, c)| {
            (i as u32, c)
        }).collect();
        fields.append(&mut scanned_classes);
        SaveFile {
            fields,
            game: self.game
        }
    }
    pub fn read_missing<R: Read + Seek>(&mut self, reader: &mut R) -> SaveFile {
        // First get the top level struct for each game/file
        // TODO: rn just for wilds, add based on game later
        let type_info = self.type_map.get_by_name("app.savedata.cUserSaveData").unwrap();
        // need to add custom class readers that also have type information along side them when
        let mut fields = Vec::new();
        if let Ok(native_field_hash) = reader.read_u32() {
            let class = self.read_class(reader, type_info).unwrap();
            if native_field_hash == 0 {
                fields.push((murmur3(&"app.savedata.cUserSaveData", 0xffffffff), class));
            } else {
                fields.push((native_field_hash, class));
            }
        }
        let type_info = self.type_map.get_by_name("via.storage.saveService.SaveFileDetail").unwrap();
        if let Ok(native_field_hash) = reader.read_u32() {
            let class = self.read_class(reader, type_info).unwrap();
            if native_field_hash == 0 {
                fields.push((murmur3(&"via.storage.saveService.SaveFileDetail", 0xffffffff), class));
            } else {
                fields.push((native_field_hash, class));
            }
        }
        SaveFile {
            fields,
            game: self.game
        }
    }

    pub fn read_with_scan<R: Read + Seek>(&mut self, reader: &mut R) -> SaveFile {
        let scanned_classes = self.scan_classes(reader);
        let fields = scanned_classes.into_iter().enumerate().map(|(i, c)| {
            (i as u32, c)
        }).collect();
        let save_file = SaveFile {
            game: self.game,
            fields
        };
        save_file
    }

    fn scan_classes<R: Read + Seek>(&mut self, reader: &mut R) -> Vec<Class> {
        let mut scanned_classes = Vec::new();
        while let Ok(value) = reader.read_u32() {
            if value == 0 {
                continue;
            }
            // first check if the value is a known hash
            let Some(type_info) = self.type_map.get_by_hash(value) else {
                continue
            };
            println!("Found class: {}, at position {}", type_info.name, reader.tell().unwrap_or(0));
            if type_info.crc == 0 || type_info.name.is_empty() {
                println!("Probably a null class, skipping");
                continue;
            }

            // backup 4 bytes to get the length field
            match reader.seek_relative(-8) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("[ERROR] Failed to seek to field length position: {e}");
                    break;
                }
            }

            let class = Class::read(reader);
            match class {
                Ok(c) => scanned_classes.push(c),
                Err(e) => eprintln!("[ERROR] Failed to read class: {e}"),
            }
        }
        scanned_classes
    }
}
