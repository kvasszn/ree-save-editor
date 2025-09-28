use std::{collections::HashSet, error::Error, fs::File, io::Write, path::{Path, PathBuf}};

use crate::{font::Oft, gensdk::Sdk, msg::Msg, pog::{Pog, PogList}, rsz::rszserde::DeRsz, scn::Scn, tex::Tex, user::User};
use serde::Serialize;

use crate::pog::{PogNode, PogPoint};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct FileReader {
    dump_sdk: bool,
    sdk_types: HashSet<u32>,

    #[allow(unused)]
    dump_all_rsz: bool,
    output_dir: std::path::PathBuf,
    root_dir: Option<std::path::PathBuf>,
    keep_path_structure: bool
}

impl FileReader {
    pub fn new(output_dir: std::path::PathBuf, root_dir: Option<std::path::PathBuf>, dump_sdk: bool, dump_all_rsz: bool, keep_path_structure: bool) -> FileReader {
        Self {
            dump_sdk,
            output_dir,
            dump_all_rsz,
            sdk_types: HashSet::new(),
            root_dir,
            keep_path_structure,
        }

    }
    
    fn get_full_file_path(&self, file_path: &std::path::Path) -> std::path::PathBuf {
        match self.root_dir {
            Some(ref root_dir) => root_dir.join(&file_path),
            None => PathBuf::from(&file_path),
        }
    }


    fn get_output_path(&self, file_path: &std::path::Path) -> std::path::PathBuf {
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
                let msg = Msg::new(file_path.to_string_lossy().to_string())?;
                msg.dump(output_path);
            },
            "user" => {
                if !is_json {
                    let file = File::open(&file_path)?;
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
                let file = File::open(file_path.clone())?;
                let tex = Tex::new(file)?;
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
            "poglst" => {
                let file = File::open(file_path.clone())?;
                let poglst = PogList::new(file)?;
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
        Ok(())
    }


    pub fn dump_files(&mut self, file_list: Vec<std::path::PathBuf>) -> Result<()> {
        file_list.iter().for_each(|file| {
            match self.dump_file(PathBuf::from(&file)) {
                Ok(_) => { 
                    println!("[INFO] Saved File {:?}", &file);
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
