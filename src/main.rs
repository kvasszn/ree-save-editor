pub mod gensdk;
pub mod align;
pub mod reerr;
pub mod bitfield;
pub mod byte_reader;
pub mod compression;
pub mod file_ext;
pub mod msg;
pub mod rsz;
pub mod tex;
pub mod user;
pub mod dersz;
pub mod pog;
pub mod font;
pub mod scn;
pub mod mesh;

extern crate image;
extern crate libdeflater;

use clap::{CommandFactory, Parser};
use dersz::{DeRsz, ENUM_FILE, RSZ_FILE};
use file_ext::{ReadExt, SeekExt};
use font::Oft;
use gensdk::Sdk;
use mesh::Mesh;
use msg::Msg;
use pog::{Pog, PogList, PogPoint, PogNode};
use rsz::Rsz;
use scn::Scn;
use serde::Serialize;
use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, read_to_string,File};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tex::Tex;
use user::User;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser, Debug)]
#[command(name = "mhtame")]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short('f'), long)]
    file_name: Option<String>,
    
    #[arg(short('r'), long)]
    root_dir: Option<String>,

    #[arg(short('l'), long)]
    list: Option<String>, 

    #[arg(short('p'), long, default_value_t = true)]
    preserve: bool,

    #[arg(short('d'), long("dump-rsz"), default_value_t = false)]
    try_dump_rsz: bool,
    
    #[arg(short('s'), long("dump-sdk"), default_value_t = false)]
    dump_sdk: bool,
    
    #[arg(short('o'), long, default_value_t = String::from("outputs"))]
    out_dir: String,
    
    #[arg(long, default_value_t = String::from("rszmhwilds.json"))]
    rsz: String,
    
    #[arg(long, default_value_t = String::from("enums.json"))]
    enums: String,
}

fn construct_paths(file: String, prefix: Option<String>, out_dir_base: String, preserve_structure: bool) -> Result<(PathBuf, PathBuf)> {
    let full_file_path = match prefix {
        Some(ref prefix) => Path::new(&prefix).join(&file),
        None => PathBuf::from(&file),
    };
    let output_path = PathBuf::from(out_dir_base).join(
        if preserve_structure {
            match prefix {
                Some(ref prefix) => {
                    let file = Path::new(&full_file_path);
                    //println!("{file:?}");
                    file.strip_prefix(prefix).unwrap().to_str().unwrap()
                }
                None => &file
            }
        } else {
            let file = Path::new(&file);
            let path = file.file_name().unwrap().to_str().unwrap();
            path
        }
    );

    Ok((full_file_path, output_path))
}

#[derive(Debug)]
enum FileType {
    Msg(u32),
    User(u32),
    Rsz,
    Scn,
    Tex(u32),
    Mesh,
    Oft,
    Pog,
    PogList,
    Mdf2,
    Sdftex,
    Annoying,
    Unknown
}

fn get_file_ext(file_name: String) -> Result<(FileType, bool)> {
    let is_json = file_name.ends_with(".json");
    let split = file_name.strip_suffix(".json").unwrap_or(&file_name).split('.').collect::<Vec<_>>();

    if split.len() < 2 {
        return Ok((FileType::Unknown, is_json))
    }
    let v = *split.get(2).unwrap_or_else(|| &"0");
    let version = match u32::from_str_radix(v, 10) {
        Ok(val) => val,
        Err(e) => {
            eprintln!("{e}?, continuing without file version extension");
            0
            //return Err(format!("{e}").into())
        },
    };

    let file_type = match split.get(1) {
        Some(ext) => {
            match *ext {
                "user" => FileType::User(version),
                "scn" => FileType::Scn,
                "msg" => FileType::Msg(version),
                "tex" => FileType::Tex(version),
                "pog" => FileType::Pog,
                "rsz" => FileType::Rsz,
                "poglst" => FileType::PogList,
                "oft" => FileType::Oft,
                "mesh" => FileType::Mesh,
                //"mdf2" => FileType::Mdf2,
                //"sdftex" => FileType::Sdftex,
                "rbs" | "sdftex" | "mcol" | "mdf2" | "mmtr" | "ocioc" | "tmlbld" | "fol" | "rmesh" | "chain2" | "jcns" | "gbpf" | "fbxskel" | "sfur" | "gtl" | "ziva" | "ucurve" | "jntexprgraph" | "vmap" | "zivacomb" | "gpbf" | "clsp" | "stmesh" | "mot" | "mpci" | "mcpi" | "chf" | "hf" | "efx" | "vsdf" | "uvar" | "rtex" | "lprb" | "gpuc" | "gui" | "prb" => FileType::Annoying,
                _ => FileType::Unknown
            }
        },
        None => {
            FileType::Unknown
        }
    };

    Ok((file_type, is_json))
}

fn dump_file(root_dir: Option<String>, file_path: PathBuf, output_path: PathBuf, dump_all_rsz: bool) -> Result<Option<HashSet<u32>>> {
    let mut res = None;
    let file_name = match file_path.file_name() {
        Some(file_name) => file_name,
        None => {
            return Err(format!("Path does not contain file").into());
        }
    };

    let (file_type, is_json )= get_file_ext(file_name.to_string_lossy().to_string())?;
    match file_type {
        FileType::Mesh | FileType::Oft | FileType::Msg(_) | FileType::Tex(_) | FileType::Annoying => return Ok(None),
        _ => ()
    }
    if dump_all_rsz {
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

    }
    //println!("{:?}, {is_json}", file_type);
    let result: Result<Option<HashSet<u32>>> = match file_type {
        FileType::Msg(_v) => {
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
            let msg = Msg::new(file_path.to_string_lossy().to_string())?;

            println!("Trying to save to {:?}", &output_path);
            let _ = fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            msg.save(&mut f);
            println!("Saved file");
            Ok(None)
        },
        FileType::User(_v) => {
            return if !is_json {
                let file = File::open(&file_path)?;
                let rsz = User::new(file)?.rsz;
                let types: HashSet<u32> = rsz.type_descriptors.iter().map(|t| t.hash).collect();
                let res = Some(types.clone());
                let nodes = rsz.deserialize()?;
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
                let json_res = serde_json::to_string_pretty(&nodes); 
                match json_res {
                    Ok(json) => {
                        let _ = fs::create_dir_all(output_path.parent().unwrap())?;
                        let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                        f.write_all(json.as_bytes())?;
                        println!("[INFO] Saved File {:?}", &output_path);
                        Ok(res)
                    },
                    Err(e) => {
                        Err(format!("File: {file_path:?}\nReason: {e}").into())
                    }
                }
            } else {
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".custom");
                let user = User::from_json_file(&file_path.to_str().unwrap())?;
                let _ = fs::create_dir_all(output_path.parent().unwrap())?;
                user.save_from_json(&output_path.to_str().unwrap())?;
                println!("{output_path:?}");
                Ok(res)
            }
        },
        FileType::Rsz => {
            return if !is_json {
                let mut file = File::open(&file_path)?;
                let rsz = Rsz::new(&mut file, 0, 0)?;
                let nodes = rsz.deserialize()?;
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
                //output_path.push(file_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
                let json_res = serde_json::to_string_pretty(&nodes); 
                match json_res {
                    Ok(json) => {
                        let _ = fs::create_dir_all(output_path.parent().unwrap())?;
                        let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                        f.write_all(json.as_bytes())?;
                        println!("[INFO] Saved File {:?}", &output_path);
                        Ok(res)
                    },
                    Err(e) => {
                        Err(format!("File: {file_path:?}\nReason: {e}").into())
                    }
                }
            } else {
                let mut output_path = output_path.clone();
                output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".custom");
                let rsz = Rsz::from_json_file(&file_path.to_str().unwrap())?;
                let _ = fs::create_dir_all(output_path.parent().unwrap())?;
                rsz.save_from_json(&output_path.to_str().unwrap())?;
                println!("{output_path:?}");
                Ok(res)
            }
        },

        FileType::Scn => {
            let file = File::open(file_path.clone())?;
            let rsz = Box::new(Scn::new(file)?.rsz);
            let  nodes = rsz.deserialize()?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");

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
                    Err(format!("File: {file_path:?}\nReason: {e}").into())
                }
            }
        },
        FileType::Tex(_v) => {
            let file = File::open(file_path.clone())?;
            let tex = Tex::new(file)?;
            let rgba = tex.to_rgba(0, 0)?;
            //println!("{}", rgba.data.len());
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".png");
            println!("saving to {output_path:?}");
            fs::create_dir_all(output_path.parent().unwrap())?;
            image::save_buffer(
                &Path::new(&output_path),
                &rgba.data,
                rgba.width,
                rgba.height,
                image::ExtendedColorType::Rgba8,
            )?;
            Ok(None)
        },
        FileType::Mesh => {
            let file = File::open(file_path.clone())?;
            let mesh = Mesh::new(file)?;
            println!("{:#?}", mesh.vertex_elements);
            //println!("{:#?}, {}, {}", mesh.vertex_elements.len(), mesh.vertex_buffer.len(), mesh.face_buffer.len());
            Ok(None)
        }
        FileType::Pog => {
            let file = File::open(file_path.clone())?;
            let pog = Pog::new(file)?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
            let nodes = pog.rszs.iter().map(|rsz| rsz.deserialize()).collect::<Result<Vec<_>>>()?;
            #[derive(Serialize)]
            struct Wrapped {
                points: Vec<PogPoint>,
                graph: Vec<PogNode>,
                nodes: Vec<DeRsz>,// confusing
            }

            let json_res = serde_json::to_string_pretty(&Wrapped {
                points: pog.points,
                graph: pog.nodes,
                nodes,
            }); 
            return match json_res {
                Ok(json) => {
                    let _ = fs::create_dir_all(output_path.parent().unwrap())?;
                    let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                    f.write_all(json.as_bytes())?;
                    println!("[INFO] Saved File {:?}", &output_path);
                    Ok(res)
                },
                Err(e) => {
                    Err(format!("File: {file_path:?}\nReason: {e}").into())
                }
            }
        },
        FileType::PogList => {
            let file = File::open(file_path.clone())?;
            let poglst = PogList::new(file)?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".json");
            let json_res = serde_json::to_string_pretty(&poglst);
            return match json_res {
                Ok(json) => {
                    let _ = fs::create_dir_all(output_path.parent().unwrap())?;
                    let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
                    f.write_all(json.as_bytes())?;
                    println!("[INFO] Saved File {:?}", &output_path);
                    Ok(res)
                },
                Err(e) => {
                    Err(format!("File: {file_path:?}\nReason: {e}").into())
                }
            }
        },
        FileType::Oft => {
            let file = File::open(file_path.clone())?;
            let oft = Oft::new(file)?;
            let mut output_path = output_path.clone();
            output_path.set_file_name(output_path.file_name().unwrap().to_str().unwrap().to_string() + ".otf");
            let _ = fs::create_dir_all(output_path.parent().unwrap())?;
            let mut f = std::fs::File::create(&output_path).expect("Error Creating File");
            f.write(&oft.data)?;
            println!("[INFO] Saved File {:?}", &output_path);
            Ok(None)
        }

        FileType::Unknown => return Err(format!("Unknown File Type {file_name:?}").into()),
        _ => return Err(format!("Annoying File Type {file_name:?}").into()),
    };
    result
}

#[allow(dead_code)]
fn find_files_with_extension(base_dir: PathBuf, extension: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut paths: Vec<PathBuf> = Vec::new();
    paths.push(base_dir);
    while let Some(dir) = paths.pop() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    paths.push(path);
                } else {
                    if let Some(x) = path.file_name().unwrap().to_str() {
                        if x.ends_with(extension) {
                            results.push(path);
                        }
                    }
                }
            }
        }
    }
    results
}

fn dump_all(root_dir: Option<String>, out_dir: String, list_file: String, dump_all_rsz: bool, dump_sdk: bool) -> Result<()> {
    let list = read_to_string(&list_file).expect("Could not open list file");
    let list: Vec<&str> = list.lines().collect();
    let mut types: HashSet<u32> = HashSet::new();
    for file in list {
        let paths = construct_paths(file.to_string(), root_dir.clone(), out_dir.clone(), true);
        let (file_path, output_path) = match paths {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[ERROR] Could not create file path {file} and output path {e}");
                continue
            }
        };
        //eprintln!("Dumping File: {file_path:?}");
        match dump_file(root_dir.clone(), file_path.clone(), output_path.clone(), dump_all_rsz) {
            Ok(res) => {
                if let Some(res) = res {
                    types.extend(res);
                }
            }
            Err(e) => {
                println!("[ERROR] File {:?}", &output_path);
                eprintln!("[ERROR] Error dumping file {e} \n\t{:?}\n\t{:?}", file_path, output_path);
                continue
            }
        };
    }
    if dump_sdk {
        let mut sdk = Sdk::new();
        sdk.add_types(types)?;
        sdk.write_files();
    }
    Ok(())
}



fn main() -> Result<()> {
    let args = Args::parse();
    // Ugly but will change later
    let rsz_file = std::env::var("RSZ_FILE").unwrap_or_else( 
        |_| {
            args.rsz
        }
    );
    if !Path::new(&rsz_file).exists() {
        eprintln!("BIG WARNING: {} not found", rsz_file);
    } 
    RSZ_FILE.set(rsz_file)?;

    let enum_file = std::env::var("ENUM_FILE").unwrap_or_else(
        |_| args.enums
    );
    if !Path::new(&enum_file).exists() {
        eprintln!("BIG WARNING: {} not found", enum_file);
    } 
    ENUM_FILE.set(enum_file)?;

    let now = SystemTime::now();
    //panic!("{:?}", args);
    match args.list {
        Some(list) => {
            dump_all(args.root_dir, args.out_dir, list, args.try_dump_rsz, args.dump_sdk)?;
        }, 
        None => match args.file_name {
            Some(file_name) => {
                let (file_path, output_path) = construct_paths(file_name.clone(), args.root_dir.clone(), args.out_dir.clone(), args.preserve)?;
                dump_file(args.root_dir, file_path, output_path, args.try_dump_rsz)?;
            },
            None => {
                println!("Please provide a file or list");
                Args::command().print_help().unwrap()
            },
        }
    }
    println!("Time taken: {} ms", now.elapsed().unwrap().as_millis());
    Ok(())
}
