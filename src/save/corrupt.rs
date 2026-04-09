use std::{
    error::Error,
    io::{Read, Seek},
};

use util::{ReadExt, SeekExt, seek_align_up};

use crate::{
    save::{
        SaveFile, SaveFlags, game::Game, types::{Array, ArrayType, Class, EnumValue, Field, FieldType, FieldValue, Struct}
    },
    sdk::{
        StringU16,
        type_map::{FieldInfo, TypeInfo, TypeMap, murmur3},
    },
};

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
        Self { type_map, game }
    }

    pub fn read_array<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        field_info: &FieldInfo,
    ) -> Result<Array, Box<dyn Error>> {
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
                        let data = (0..size).map(|_| Ok(reader.read_u16()?)).collect::<Result<
                            Vec<u16>,
                            Box<dyn Error>,
                        >>(
                        )?;
                        FieldValue::String(Box::new(StringU16::new(data)))
                    } else {
                        self.read_field_value_sized(reader, member_type, member_size, field_info)?
                    }
                }
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
                }
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
            values,
        })
    }

    pub fn read_field_value<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        field_type: FieldType,
        field_info: &FieldInfo,
    ) -> Result<FieldValue, Box<dyn Error>> {
        let value = match field_type {
            FieldType::Unknown => {
                /*println!("Unknown Field Type found");*/
                return Err("Unknown Field Type".into());
            }
            FieldType::Array => FieldValue::Array(self.read_array(reader, field_info)?.into()),
            FieldType::Class => {
                // TODO: remove unwrap
                let type_info = field_info.get_original_type(self.type_map).unwrap();
                FieldValue::Class(self.read_class(reader, type_info)?.into())
            }
            FieldType::String => {
                reader.seek_align_up(4)?;
                let size = reader.read_u32()?;
                let data = (0..size)
                    .map(|_| Ok(reader.read_u16()?))
                    .collect::<Result<Vec<u16>, Box<dyn Error>>>()?;
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
    pub fn read_field_value_sized<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        field_type: FieldType,
        size: u32,
        field_info: &FieldInfo,
    ) -> Result<FieldValue, Box<dyn Error>> {
        let pos = reader.stream_position()?;
        let size = if field_type != FieldType::String && field_type != FieldType::Struct 
            && size != field_info.size as u32 {
            eprintln!("[WARNING] invalid size={size:x}, good={:x} for field {}: {}: @{pos:#x}", field_info.size, field_info.name, field_info.original_type);
            field_info.size as u32
        } else {
            size
        };
        if field_type != FieldType::String {
            reader.seek_align_up(size as u64)?;
        }
        let value = match field_type {
            FieldType::Enum => {
                let enum_val = match size {
                    1 => EnumValue::E1(reader.read_i8()?),
                    2 => EnumValue::E2(reader.read_i16()?),
                    4 => EnumValue::E4(reader.read_i32()?),
                    8 => EnumValue::E8(reader.read_i64()?),
                    _ => return Err(format!("Invalid Enum size: {}", size).into()),
                };
                FieldValue::Enum(enum_val)
            },
            FieldType::Boolean => FieldValue::Boolean(reader.read_bool()?),
            FieldType::S8 => FieldValue::S8(reader.read_i8()?),
            FieldType::U8 => FieldValue::U8(reader.read_u8()?),
            FieldType::S16 => FieldValue::S16(reader.read_i16()?),
            FieldType::U16 => FieldValue::U16(reader.read_u16()?),
            FieldType::S32 => FieldValue::S32(reader.read_i32()?),
            FieldType::U32 => FieldValue::U32(reader.read_u32()?),
            FieldType::S64 => FieldValue::S64(reader.read_i64()?),
            FieldType::U64 => FieldValue::U64(reader.read_u64()?),
            FieldType::F32 => FieldValue::F32(reader.read_f32()?),
            FieldType::F64 => FieldValue::F64(reader.read_f64()?),
            FieldType::C8 => FieldValue::C8(reader.read_u8()?),
            FieldType::C16 => FieldValue::C16(reader.read_u16()?),
            FieldType::Array => FieldValue::Array(self.read_array(reader, field_info)?.into()),
            FieldType::Struct => {
                let mut data = vec![0u8; size as usize];
                reader.read_exact(&mut data)?;
                FieldValue::Struct(Box::new(Struct { data }))
            }
            _ => {
                return Err(
                    format!("Unexpected sized read of {:?} for {:?}", size, field_type).into(),
                );
            }
        };
        return Ok(value);
    }

    pub fn read_field<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        parent: &TypeInfo,
        field: &FieldInfo,
    ) -> crate::file::Result<Field> {
        let _hash = reader.read_u32()?;
        let field_type_raw = reader.read_i32()?;
        let field_type = FieldType::try_from(field_type_raw);
        let good_field_type = FieldType::from_field_info(field);
        let pos = reader.stream_position()?;
        let mut field_type = field_type.unwrap_or_else(|e| {
            eprintln!("[WARNING] Invalid Field Type {field_type_raw:x} @{pos:#x}, using good_file_type={good_field_type:?}: {e:?}");
            good_field_type
        });

        if field_type != good_field_type && good_field_type != FieldType::Struct {
            eprintln!(
                "[WARNING] Field type mismatch good={good_field_type:?}@{pos:#x}, read={field_type_raw:?} in field {} from class {}",
                field.name, parent.name
            );
            field_type = good_field_type;
        }
        let value = self.read_field_value(reader, field_type, field)?;
        seek_align_up(reader, 4)?;
        Ok(Field {
            hash: field.hash,
            field_type,
            value,
        })
    }

    pub fn read_class_safe<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        type_info: &TypeInfo,
    ) -> crate::file::Result<Class> {
        let num_fields = match reader.read_u32() {
            Err(e) => {
                eprintln!(
                    "[WARNING] Could not read num fields in class {}: {e}",
                    type_info.name
                );
                return Err(Box::new(e));
                //return Ok(Class { num_fields: 0, fields: IndexMap::new(), hash: 0})
            }
            Ok(r) => r,
        };
        let mut hash = match reader.read_u32() {
            Err(e) => {
                eprintln!(
                    "[WARNING] Could not read hash in class {}: {e}",
                    type_info.name
                );
                return Err(Box::new(e));
                //return Ok(Class { num_fields, fields: IndexMap::new(), hash: 0})
            }
            Ok(r) => r,
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

        let mut fields = Vec::<Field>::new();
        // could maybe do something where you keep on trying to read a field hash until you cant
        // could also maybe just read in order of the RSZ, if its there, its there, otherwise just
        // ignore
        for (field_hash, field_info) in &type_info.fields {
            let read_field_hash = match reader.read_u32() {
                Err(e) => {
                    eprintln!(
                        "[WARNING] Could not read field hash in class {} for field {}: {e}",
                        type_info.name, field_info.name
                    );
                    return Err(Box::new(e));
                }
                Ok(r) => r,
            };
            let _ = reader.seek_relative(-4);
            if *field_hash != read_field_hash && read_field_hash != 0 {
                //continue;
            }

            let field = self.read_field(reader, type_info, field_info);
            match field {
                Ok(field) => {
                    fields.push(field);
                }
                Err(e) => {
                    eprintln!("[ERROR] Parsing error on field {}, {e}", field_info.name);
                    return Err(e);
                }
            }
        }

 
        Ok(Class {
            num_fields,
            hash,
            fields,
        })
    }

    // this breaks on polymorphism
    pub fn read_class<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        type_info: &TypeInfo,
    ) -> crate::file::Result<Class> {
        let num_fields = match reader.read_u32() {
            Err(e) => {
                eprintln!(
                    "[WARNING] Could not read num fields in class {}: {e}",
                    type_info.name
                );
                return Err(Box::new(e));
                //return Ok(Class { num_fields: 0, fields: IndexMap::new(), hash: 0})
            }
            Ok(r) => r,
        };
        let mut hash = match reader.read_u32() {
            Err(e) => {
                eprintln!(
                    "[WARNING] Could not read hash in class {}: {e}",
                    type_info.name
                );
                return Err(Box::new(e));
                //return Ok(Class { num_fields, fields: IndexMap::new(), hash: 0})
            }
            Ok(r) => r,
        };
        let pos = reader.stream_position()?;
        let correct_hash = murmur3(&type_info.name, 0xffffffff);
        if hash != correct_hash {
            eprintln!("[WARNING] different hashes correct_hash={correct_hash:x}, read_hash={hash:x}: @{pos:#x}");
            hash = correct_hash;
        }

        let num_fields = if num_fields == 0 {
            type_info.fields.len() as u32
        } else {
            num_fields.max(type_info.fields.len() as u32)
        };

        let mut fields = Vec::<Field>::new();
        // could maybe do something where you keep on trying to read a field hash until you cant
        // could also maybe just read in order of the RSZ, if its there, its there, otherwise just
        // ignore
        for (field_hash, field_info) in &type_info.fields {
            let read_field_hash = match reader.read_u32() {
                Err(e) => {
                    eprintln!(
                        "[WARNING] Could not read field hash in class {} for field {}: {e}",
                        type_info.name, field_info.name
                    );
                    *field_hash
                        /*return Ok(Class {
                          num_fields,
                          hash,
                          fields,
                          });*/
                }
                Ok(r) => r,
            };
            let _ = reader.seek_relative(-4);
            if *field_hash != read_field_hash && read_field_hash != 0 {
                //continue;
            }

            let field = self.read_field(reader, type_info, field_info);
            match field {
                Ok(field) => {
                    fields.push(field);
                }
                Err(e) => {
                    eprintln!("[ERROR] Parsing error on field {}, {e}", field_info.name);
                    return Ok(Class {
                        num_fields,
                        hash,
                        fields,
                    });
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
            fields,
        })
    }

    pub fn read_missing_and_scan<R: Read + Seek>(&mut self, reader: &mut R) -> SaveFile {
        // First get the top level struct for each game/file
        // TODO: rn just for wilds, add based on game later
        let type_info = self
            .type_map
            .get_by_name("app.savedata.cUserSaveData")
            .unwrap();
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
        let type_info = self
            .type_map
            .get_by_name("via.storage.saveService.SaveFileDetail")
            .unwrap();
        if let Ok(native_field_hash) = reader.read_u32() {
            let class = self.read_class(reader, type_info).unwrap();
            if native_field_hash == 0 {
                fields.push((
                        murmur3(&"via.storage.saveService.SaveFileDetail", 0xffffffff),
                        class,
                ));
            } else {
                fields.push((native_field_hash, class));
            }
        }
        /*reader.seek(std::io::SeekFrom::Start(0)).unwrap();
          let scanned_class = self.scan_class_fields(reader, murmur3(&"app.savedata.cUserSaveParam", 0xffffffff), 0);
          fields.push((fields.len() as u32, scanned_class));

          let pos = reader.stream_position().unwrap();
          let scanned_class = self.scan_class_fields(reader, murmur3(&"app.savedata.cUserSaveParam", 0xffffffff), pos);
          fields.push((fields.len() as u32, scanned_class));

          let pos = reader.stream_position().unwrap();
          let scanned_class = self.scan_class_fields(reader, murmur3(&"app.savedata.cUserSaveParam", 0xffffffff), pos);
          fields.push((fields.len() as u32, scanned_class));*/
        let scanned_classes = self.scan_classes(reader);
        let mut scanned_classes: Vec<(u32, Class)> = scanned_classes
            .into_iter()
            .enumerate()
            .map(|(i, c)| (i as u32, c))
            .collect();
        fields.append(&mut scanned_classes);
        SaveFile {
            fields,
            flags: SaveFlags::game_default(self.game),
            game: self.game,
        }
    }

    pub fn read_n_objects<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        name: &str,
        n: u64,
    ) -> SaveFile {
        let mut fields = Vec::new();
        reader.seek(std::io::SeekFrom::Start(0)).unwrap();

        let hash = murmur3(name, 0xffffffff);
        for _ in 0..n {
            let pos = reader.stream_position().unwrap();
            let scanned_class = self.scan_class_fields(reader, hash, pos);
            fields.push((fields.len() as u32, scanned_class));
        }

        SaveFile {
            fields,
            flags: SaveFlags::game_default(self.game),
            game: self.game,
        }
    }

    pub fn read_missing<R: Read + Seek>(&mut self, reader: &mut R, types: &[(u32, &str)]) -> SaveFile {
        let mut fields = Vec::new();
        for (field_hash, type_name) in types {
            let type_info = self
                .type_map
                .get_by_name(&type_name)
                .unwrap();
            if let Ok(native_field_hash) = reader.read_u32() {
                let class = self.read_class(reader, type_info).unwrap();
                if native_field_hash != *field_hash {
                    eprintln!("[WARNING] Top level native field hash {native_field_hash:#x} != {field_hash:#x}");
                }
                fields.push((*field_hash, class));
            }

        }
        SaveFile {
            fields,
            flags: SaveFlags::game_default(self.game),
            game: self.game,
        }
    }

    pub fn read_with_scan<R: Read + Seek>(&mut self, reader: &mut R) -> SaveFile {
        let scanned_classes = self.scan_classes(reader);
        let fields = scanned_classes
            .into_iter()
            .enumerate()
            .map(|(i, c)| (i as u32, c))
            .collect();
        let save_file = SaveFile {
            game: self.game,
            flags: SaveFlags::game_default(self.game),
            fields,
        };
        save_file
    }

    fn scan_class_fields<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        hash: u32,
        start: u64,
    ) -> Class {
        let mut fields = Vec::new();
        let Some(type_info) = self.type_map.get_by_hash(hash) else {
            return Class {
                num_fields: 0,
                hash,
                fields,
            };
        };
        for (field_hash, field_info) in &type_info.fields {
            // scan for the specific field
            reader.seek(std::io::SeekFrom::Start(start)).unwrap();

            //println!("[INFO] Checking {}", field_info.name);
            let original_type_hash = murmur3(&field_info.original_type, 0xffffffff);
            if field_info.r#type == "Object" && !field_info.array {
                while let Ok(value) = reader.read_u32() {
                    if value == original_type_hash {
                        match reader.seek_relative(-8) {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("[ERROR] Failed to seek to field length position: {e}");
                                break;
                            }
                        }
                        let type_info = self.type_map.get_by_name(&field_info.original_type);
                        let Some(type_info) = type_info else {
                            break;
                        };
                        //let class = self.read_class_safe(reader, type_info);
                        let class = Class::read(reader);
                        match class {
                            Ok(c) => {
                                println!(
                                    "[INFO] Found field {} in class {}",
                                    field_info.name, type_info.name
                                );
                                let field = Field {
                                    field_type: FieldType::Class,
                                    hash: *field_hash,
                                    value: FieldValue::Class(Box::new(c)),
                                };
                                fields.push(field);
                                //scanned_classes.push(c);
                            }
                            Err(e) => {
                                eprintln!("[ERROR] Failed to read class: {e}")
                            }
                        }
                        break;
                    }
                }
            } else {
                while let Ok(value) = reader.read_u32() {
                    if value == *field_hash {
                        match reader.seek_relative(-4) {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("[ERROR] Failed to seek to field hash position: {e}");
                                break;
                            }
                        }
                        let field = Field::read(reader);
                        match field {
                            Ok(f) => {
                                println!(
                                    "[INFO] Found field {} in class {}",
                                    field_info.name, type_info.name
                                );
                                fields.push(f);
                            }
                            Err(e) => {
                                eprintln!("[ERROR] Failed to read field: {e}")
                            }
                        }
                        break;
                    }
                }
            }
        }
        return Class {
            num_fields: fields.len() as u32,
            hash,
            fields,
        };
    }

    fn scan_classes<R: Read + Seek>(&mut self, reader: &mut R) -> Vec<Class> {
        let mut scanned_classes = Vec::new();
        while let Ok(value) = reader.read_u32() {
            let start = reader.stream_position().unwrap();
            if value == 0 {
                continue;
            }
            // first check if the value is a known hash
            let Some(type_info) = self.type_map.get_by_hash(value) else {
                continue;
            };
            println!(
                "Found class: {}, at position {}",
                type_info.name,
                reader.tell().unwrap_or(0)
            );
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
                Err(e) => {
                    reader.seek(std::io::SeekFrom::Start(start)).unwrap();
                    eprintln!("[ERROR] Failed to read class: {e}")
                }
            }
        }
        scanned_classes
    }
}
