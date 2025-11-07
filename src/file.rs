use std::{collections::HashSet, error::Error, fs::File, io::{Cursor, Read, Seek, Write}, mem::MaybeUninit, path::{Path, PathBuf}, time::SystemTime};

use crate::{file_ext::SeekExt, font::Oft, gensdk::Sdk, msg::Msg, pog::{Pog, PogList}, rsz::rszserde::{DeRsz, Guid, StringU16}, save::{types::to_dersz, SaveContext}, scn::Scn, tdb::TDBHeader, tex::Tex, user::User};
use crate::save::SaveFile;
use serde::Serialize;

use crate::pog::{PogNode, PogPoint};

use rayon::prelude::*;
use std::sync::{Arc, Mutex};

#[macro_export]
macro_rules! sread {
    ($t:ty) => {
        <$ty>::read(reader, &mut ())
    }
}

pub trait Dump<R = ()> {
    fn dump(file_path: &Path, output_path: &Path) -> Result<R>;
}

pub trait DefaultDump {}

impl<T: StructRW + DefaultDump + Serialize> Dump for T {
    fn dump(file_path: &Path, output_path: &Path) -> Result<()> {
        let mut file = File::open(file_path)?;
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        let mut reader = Cursor::new(buf);
        let res = T::read(&mut reader, &mut ())?;
        let mut output_path = output_path.to_path_buf();
        output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
        let json_res = serde_json::to_string_pretty(&res)?;
        std::fs::create_dir_all(output_path.parent().unwrap())?;
        let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
        f.write_all(json_res.as_bytes())?;
        Ok(())
    }
}


pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub trait StructRW<C = ()>{
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut C) -> Result<Self>
        where
            Self: Sized;
}

#[derive(Debug)]
pub struct Magic<const N: usize>(pub [u8; N]);

impl<const N: usize> PartialEq<[u8; N]> for Magic<N> {
    fn eq(&self, other: &[u8; N]) -> bool {
        &self.0 == other
    }
    fn ne(&self, other: &[u8; N]) -> bool {
        &self.0 != other
    }
}

impl<const N: usize> Serialize for Magic<N> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let s = String::from_utf8((&self.0).to_vec()).unwrap();
        serializer.serialize_str(&s)
    }
}

impl <C, const N: usize> StructRW<C> for Magic<N> {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut C) -> Result<Self>
            where
                Self: Sized {
        Ok(Magic(<[u8; N]>::read(reader, ctx)?))
    }
}

impl <C, T: StructRW<C>> StructRW<C> for Option<T> {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut C) -> Result<Self>
            where
                Self: Sized {
        Ok(Some(T::read(reader, ctx)?))
    }
}

macro_rules! derive_primitives {
    ($($ty:ty),*) => {
        $(
            impl<C> StructRW<C> for $ty {
                fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut C) -> Result<Self> {
                    use std::mem::size_of;
                    let mut buf = [0u8; size_of::<$ty>()];
                    reader.read_exact(&mut buf)?;
                    Ok(<$ty>::from_le_bytes(buf))
                }
            }
        )*
    };
}

derive_primitives!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl<C, T: StructRW<C>, const N: usize> StructRW<C> for [T; N] {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut C) -> Result<Self>
            where
                Self: Sized {
                    let mut arr: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

                    for i in 0..N {
                        arr[i].write(T::read(reader, ctx)?);
                    }

                    // Convert from [MaybeUninit<T>] to [T] safely
                    Ok(unsafe { std::mem::transmute_copy::<[MaybeUninit<T>; N], [T; N]>(&arr) })
    }
}

impl StructRW<usize> for Vec<u8> {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut usize) -> Result<Self> {
        let mut values = vec![0u8; *ctx];
        reader.read_exact(&mut values)?;
        Ok(values)
    }
}

impl<C> StructRW<C> for Guid {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut C) -> Result<Self>
    where
        Self: Sized {
            Ok(Guid(<[u8; 16]>::read(reader, ctx)?))
    }
}

impl<C> StructRW<C> for StringU16 {
    fn read<R: Read + Seek>(reader: &mut R, ctx: &mut C) -> Result<Self> {
        let mut u16str = vec![];
        loop {
            let c = <u16>::read(reader, ctx)?;
            if c == 0 {
                break;
            }
            u16str.push(c);
        }
        Ok(StringU16(u16str))
    }
}

#[derive(Debug, Serialize)]
pub struct StringU16Offset {
    offset: u64,
    value: String,
}

impl<C> StructRW<C> for StringU16Offset {
    fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut C) -> Result<Self>
    where
        Self: Sized {
            let offset = u64::read(reader, &mut ())?;
            let pos = reader.tell()?;
            reader.seek(std::io::SeekFrom::Start(offset.into()))?;
            let mut u16str = vec![];
            loop {
                let c = <u16>::read(reader, &mut ())?;
                if c == 0 {
                    break;
                }
                u16str.push(c);
            }
            reader.seek(std::io::SeekFrom::Start(pos.into()))?;
            Ok(Self {
                offset,
                value: String::from_utf16(&u16str)?
            })
        }
}

#[derive(Debug)]
pub struct StringOffset {
    offset: u64,
    value: String,
}

impl<C> StructRW<C> for StringOffset {
    fn read<R: Read + Seek>(reader: &mut R, _ctx: &mut C) -> Result<Self>
    where
        Self: Sized {
            let offset = u64::read(reader, &mut ())?;
            let pos = reader.tell()?;
            reader.seek(std::io::SeekFrom::Start(offset.into()))?;
            let mut u8str = vec![];
            loop {
                let c = <u8>::read(reader, &mut ())?;
                if c == 0 {
                    break;
                }
                u8str.push(c);
            }
            reader.seek(std::io::SeekFrom::Start(pos.into()))?;
            Ok(Self {
                offset,
                value: String::from_utf8(u8str)?
            })
        }
}

/*#[derive(StructRW)]
  pub struct UserStrRWTest {
  magic: u32,
  version: u32,
  resource_count: u32,
  child_count: u32,
  padding: u32,
  }
  */

pub struct FileReader {
    dump_sdk: bool,
    sdk_types: HashSet<u32>,
    steamid: Option<String>,

    #[allow(unused)]
    dump_all_rsz: bool,
    output_dir: std::path::PathBuf,
    root_dir: Option<std::path::PathBuf>,
    keep_path_structure: bool
}

impl FileReader {
    
    pub fn new(output_dir: std::path::PathBuf, root_dir: Option<std::path::PathBuf>, dump_sdk: bool, dump_all_rsz: bool, keep_path_structure: bool, steamid: Option<String>) -> FileReader {
        Self {
            dump_sdk,
            output_dir,
            dump_all_rsz,
            sdk_types: HashSet::new(),
            root_dir,
            steamid,
            keep_path_structure,
        }

    }

    pub fn get_full_file_path(&self, file_path: &std::path::Path) -> std::path::PathBuf {
        match self.root_dir {
            Some(ref root_dir) => root_dir.join(&file_path),
            None => PathBuf::from(&file_path),
        }
    }


    pub fn get_output_path(&self, file_path: &std::path::Path) -> std::path::PathBuf {
        let path = if self.keep_path_structure {
            match self.root_dir {
                Some(ref prefix) => {
                    let file = &self.get_full_file_path(file_path);
                    file.strip_prefix(prefix).unwrap().to_path_buf()
                }
                None => file_path.to_path_buf()
            }
        } else {
            let file = Path::new(&file_path);
            let path = file.file_name().unwrap().to_str().unwrap();
            PathBuf::from(path)
        };
        self.output_dir.join(path)
    }

    pub fn dump_file(&mut self, file: std::path::PathBuf) -> Result<()> {
        let file_path = self.get_full_file_path(&file);
        let output_path = self.get_output_path(&file);
        println!("[INFO] Dumping File: {file_path:?}");

        let file_name = file_path.file_name().ok_or(format!("Path does not contain file"))?.to_string_lossy();

        let split = file_name.strip_suffix(".json").unwrap_or(&file_name).split('.').collect::<Vec<_>>();
        let is_json = file_name.ends_with(".json");
        let file_ext = *split.get(1).ok_or(format!("Could not determine file type from file name"))?;

        match file_ext {
            "msg" => {
                Msg::dump(&file_path, &output_path)?;
            },
            "poglst" => {
                PogList::dump(&file_path, &output_path)?;
            }
            "bin" => {
                if let Some(steamid) = &self.steamid {
                    let steamid = if let Some(hex) = steamid.strip_prefix("0x") {
                        u64::from_str_radix(hex, 16)
                    } else {
                        u64::from_str_radix(steamid, 10)
                    }?;
                    //Mandarin::sanity_check(&file_path);
                    let mut reader = File::open(&file)?;
                    let save = SaveFile::read(&mut reader, &mut SaveContext{key: steamid})?;
                    //let save = SaveFile::from_file(&file)?;
                    let dersz = to_dersz(save.data)?;
                    //println!("{:?}, {:?}", dersz.structs.len(), dersz.roots);
                    let mut output_path = output_path.clone();
                    output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".json");
                    //println!("{output_path:?}");
                    std::fs::create_dir_all(output_path.parent().unwrap())?;
                    let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                    let json = serde_json::to_string_pretty(&dersz)?;
                    f.write_all(json.as_bytes())?;
                } else {

                    // try to brute force
                    let mut reader = File::open(&file).unwrap();
                    let mut buf = vec![];
                    reader.read_to_end(&mut buf).unwrap();
                    let mut reader = Cursor::new(&buf);
                    let _ = SaveFile::read(&mut reader, &mut SaveContext{key: 0});
                    /*(0..c).into_par_iter().for_each(|i| {
                        let key = 0x0110000100000000 + i;
                        let mut reader = Cursor::new(&buf);
                        let save = match SaveFile::read(&mut reader, &mut SaveContext{key}) {
                            Ok(save) => {
                                println!("decrypted with key={:#18x}", key);
                                //let save = SaveFile::from_file(&file)?;
                                let dersz = DeRsz::from(save);
                                //println!("{:?}, {:?}", dersz.structs.len(), dersz.roots);
                                let mut output_path = output_path.clone();
                                output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".json");
                                let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                                let json = serde_json::to_string_pretty(&dersz).unwrap();
                                f.write_all(json.as_bytes()).unwrap();
                            },
                            Err(_) => { }
                        };
                    });*/
                    return Err(format!("Can only decrypt save files with steamid.\nGo here to find it https://help.steampowered.com/en/faqs/view/2816-BE67-5B69-0FEC").into())
                }
            }
            "exe" => {
                let mut file = File::open(&file_path)?;
                TDBHeader::from_exe(&mut file)?;

            }
            "user" => {
                if !is_json {
                    let mut file = File::open(&file_path)?;
                    //let user_test = UserTest::read(&mut file, &mut ());
                    //println!("{:?}", user_test);
                    file.seek(std::io::SeekFrom::Start(0))?;
                    let user = User::new(file)?;
                    self.sdk_types.extend(user.rsz.type_descriptors.iter().map(|t| t.hash));
                    let mut output_path = output_path.clone();
                    output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".json");
                    let json = serde_json::to_string_pretty(&user)?;
                    std::fs::create_dir_all(output_path.parent().unwrap())?;
                    let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                    f.write_all(json.as_bytes())?;
                } else {
                    let mut output_path = output_path.clone();
                    output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".custom");
                    let user = User::from_json_file(&file_path.to_str().unwrap())?;
                    std::fs::create_dir_all(output_path.parent().unwrap())?;
                    let mut output = File::create(&output_path)?;
                    output.write_all(&user.to_buf()?)?;
                }
            }
            "tex" => {
                let mut file = File::open(file_path.clone())?;
                //let mut buf = vec![];
                //file.read_to_end(&mut buf)?;
                //let mut reader = Cursor::new(buf);
                let tex = Tex::new(&mut file)?;
                let rgba = tex.to_rgba(0, 0)?;
                //println!("{}", rgba.data.len());
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".png");
                std::fs::create_dir_all(output_path.parent().unwrap())?;
                image::save_buffer(
                    &output_path,
                    &rgba.data,
                    rgba.width,
                    rgba.height,
                    image::ExtendedColorType::Rgba8,
                )?;
            }
            "scn" => {
                let file = File::open(file_path.clone())?;
                let rsz = Box::new(Scn::new(file)?.rsz);
                let nodes = rsz.deserialize_to_dersz()?;
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
                let json_res = serde_json::to_string_pretty(&nodes)?; 
                let _ = std::fs::create_dir_all(output_path.parent().unwrap())?;
                let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                f.write_all(json_res.as_bytes())?;
            }
            "pog" => {
                let file = File::open(file_path.clone())?;
                let pog = Pog::new(file)?;
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
                let mut nodes = Vec::new();
                for rsz in pog.rszs {
                    self.sdk_types.extend(rsz.type_descriptors.iter().map(|t| t.hash));
                    nodes.push(rsz.deserialize_to_dersz()?);
                }
                #[derive(Serialize)]
                struct Wrapped {
                    points: Vec<PogPoint>,
                    graph: Vec<PogNode>,
                    nodes: Vec<DeRsz>,// confusing
                }

                let json_res = serde_json::to_string_pretty(&Wrapped {
                    points: pog.points,
                    graph: pog.nodes,
                    nodes: nodes
                })?; 

                std::fs::create_dir_all(output_path.parent().unwrap())?;
                let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                f.write_all(json_res.as_bytes())?;
            },
            "oft" => {
                let file = File::open(file_path.clone())?;
                let oft = Oft::new(file)?;
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".otf");
                std::fs::create_dir_all(output_path.parent().unwrap())?;
                let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                f.write(&oft.data)?;
            }
            _ => { }
        }

        println!("[INFO] Saved File {:?}", output_path);
        Ok(())
    }


    pub fn dump_files(&mut self, file_list: Vec<std::path::PathBuf>) -> Result<()> {
        file_list.iter().for_each(|file| {
            match self.dump_file(PathBuf::from(&file)) {
                Ok(_) => { 
                }
                Err(e) => {
                    eprintln!("[ERROR] Error dumping file {:?}: {e}", self.get_full_file_path(file));
                }
            };

        });
        if self.dump_sdk {
            let mut sdk = Sdk::new();
            sdk.add_types(self.sdk_types.clone())?;
            sdk.write_files();
        }
        Ok(())
    }

    pub fn dump_filesv2(&mut self, file_list: Vec<std::path::PathBuf>) -> Result<()> {
        let this = Arc::new(Mutex::new(self));
        file_list.par_iter().for_each(|file| {
            if let Err(e) = dump_file_simple(file, this.clone()) {
                eprintln!("[ERROR] Error dumping file {:?}: {e}", file);
            }
        });
        Ok(())
    }

}

pub fn dump_file_simple<'a>(file: &std::path::Path, ctx: Arc<Mutex<&'a mut FileReader>>) -> Result<()> {
    let (file_path, output_path) = {
        let ctx = ctx.lock().unwrap();
        (ctx.get_full_file_path(&file), ctx.get_output_path(&file))
    };

    println!("[INFO] Dumping File: {file_path:?}");

    let file_name = file_path.file_name().ok_or(format!("Path does not contain file"))?.to_string_lossy();

    let split = file_name.strip_suffix(".json").unwrap_or(&file_name).split('.').collect::<Vec<_>>();
    let is_json = file_name.ends_with(".json");
    let file_ext = *split.get(1).ok_or(format!("Could not determine file type from file name"))?;

    match file_ext {

        "exe" => {
            let mut file = File::open(&file_path)?;
            TDBHeader::from_exe(&mut file)?;

        }
        "msg" => {
            let mut file = File::open(&file_path)?;
            let mut buf = vec![];
            file.read_to_end(&mut buf)?;
            let mut reader = Cursor::new(buf);
            let msg = Msg::read(&mut reader, &mut ()).unwrap();
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".json");
            let json = serde_json::to_string_pretty(&msg)?;
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            f.write_all(json.as_bytes())?;
        }
        "user" => {
            if !is_json {
                let mut file = File::open(&file_path)?;
                //let user_test = UserTest::read(&mut file, &mut ());
                //println!("{:?}", user_test);
                file.seek(std::io::SeekFrom::Start(0))?;
                let user = User::new(file)?;
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".json");
                let json = serde_json::to_string_pretty(&user)?;
                std::fs::create_dir_all(output_path.parent().unwrap())?;
                let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                f.write_all(json.as_bytes())?;
                let mut ctx = ctx.lock().unwrap();
                ctx.sdk_types.extend(user.rsz.type_descriptors.iter().map(|t| t.hash));
            } else {
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_string_lossy().to_string() + ".custom");
                let user = User::from_json_file(&file_path.to_str().unwrap())?;
                std::fs::create_dir_all(output_path.parent().unwrap())?;
                let mut output = File::create(&output_path)?;
                output.write_all(&user.to_buf()?)?;
            }
        }
        "tex" => {
            let mut file = File::open(file_path.clone())?;
            //let mut buf = vec![];
            //file.read_to_end(&mut buf)?;
            //let mut reader = Cursor::new(buf);
            let tex = Tex::new(&mut file)?;
            let rgba = tex.to_rgba(0, 0)?;
            //println!("{}", rgba.data.len());
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".png");
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            image::save_buffer(
                &output_path,
                &rgba.data,
                rgba.width,
                rgba.height,
                image::ExtendedColorType::Rgba8,
            )?;
        }
        "scn" => {
            let file = File::open(file_path.clone())?;
            let rsz = Box::new(Scn::new(file)?.rsz);
            let nodes = rsz.deserialize_to_dersz()?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
            let json_res = serde_json::to_string_pretty(&nodes)?; 
            let _ = std::fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            f.write_all(json_res.as_bytes())?;
        }
        "pog" => {
            let file = File::open(file_path.clone())?;
            let pog = Pog::new(file)?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
            let mut nodes = Vec::new();
            {
                let mut ctx = ctx.lock().unwrap();
                for rsz in pog.rszs {
                    ctx.sdk_types.extend(rsz.type_descriptors.iter().map(|t| t.hash));
                    nodes.push(rsz.deserialize_to_dersz()?);
                }
            }
            #[derive(Serialize)]
            struct Wrapped {
                points: Vec<PogPoint>,
                graph: Vec<PogNode>,
                nodes: Vec<DeRsz>,// confusing
            }

            let json_res = serde_json::to_string_pretty(&Wrapped {
                points: pog.points,
                graph: pog.nodes,
                nodes: nodes
            })?; 

            std::fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            f.write_all(json_res.as_bytes())?;
        },
        "poglst" => {
            let mut file = File::open(&file_path)?;
            let poglst = PogList::read(&mut file, &mut ())?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
            let json_res = serde_json::to_string_pretty(&poglst)?;
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            f.write_all(json_res.as_bytes())?;
        }
        "oft" => {
            let file = File::open(file_path.clone())?;
            let oft = Oft::new(file)?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".otf");
            std::fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            f.write(&oft.data)?;
        }
        _ => { }
    }

    println!("[INFO] Saved File {:?}", output_path);
    Ok(())
}


/*if dump_all_rsz {
  println!("dump all rsz file {file_path:?}");
  let mut rszs: Vec<Rsz> = Vec::new();
  let mut file = File::open(file_path.clone())?;
  let target = ['R', 'S', 'Z', '\0'];
  let mut target_idx = 0;
  let len = file.seek(std::io::SeekFrom::End(0))?;
  file.seek(std::io::SeekFrom::Start(0))?;
  loop {
  let byte = file.read_u8().expect("FUCK");
//println!("{idx}:{byte} {target_idx}");
if byte == target[target_idx] as u8 {
target_idx += 1;
if target_idx == 4 {
println!("found_pattern");
let start = file.tell()? - 4;
match Rsz::new(&mut file, start, 0) {
Ok(rsz) => {
rszs.push(rsz)
},
Err(e) => {
eprintln!("Error trying to parse all rsz in file {file_path:?}");
},
}
target_idx = 0;
}
} else {
target_idx = 0;
}
if file.tell()? >= len {
break
}
}
if rszs.len() > 0 {
//println!("{rszs:#?}\n");
let mut output_path = output_path.clone();
output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
let nodes = rszs.iter().map(|rsz| rsz.deserialize()).collect::<Result<Vec<_>>>()?;
let json_res = serde_json::to_string_pretty(&nodes); 
return match json_res {
Ok(json) => {
let _ = fs::create_dir_all(output_path.parent().unwrap())?;
let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
f.write_all(json.as_bytes())?;
println!("[INFO] Saved File {:?}", &output_path);
Ok(res)
},
Err(e) => {
//eprintln!("File: {file_path:?}\nReason: {e}");
Err(format!("File: {file_path:?}\nReason: {e}").into())
}
}
}
return Ok(res)
}*/
