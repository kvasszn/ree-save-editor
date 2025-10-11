pub mod gensdk;
pub mod align;
pub mod reerr;
pub mod bitfield;
pub mod compression;
pub mod file_ext;
pub mod msg;
pub mod rsz;
pub mod tex;
pub mod user;
pub mod pog;
pub mod font;
pub mod scn;
pub mod mesh;
pub mod file;
pub mod save;
pub mod crypt;
pub mod tdb;

extern crate image;
extern crate libdeflater;

use clap::Parser;
use file::FileReader;
use rsz::dump::{ENUM_FILE, RSZ_FILE};

use std::error::Error;
use std::fs::{read_to_string};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

    #[arg(long)]
    save_file: Option<PathBuf>,

    #[arg(long)]
    steamid: Option<String>,
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

fn main() -> Result<()> {
    env_logger::init();
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
    let mut list: Vec<PathBuf> = Vec::new();
    if let Some(lf) = &args.list {
        let lf = read_to_string(&lf).expect("Could not open list file");
        for line in lf.lines() {
            list.push(PathBuf::from(line));
        }
    }

    if let Some(f) = &args.file_name {
        list.push(PathBuf::from(f));
    }

    let mut file_reader = FileReader::new(args.out_dir.into(), args.root_dir.map(|x| PathBuf::from(x)), args.dump_sdk, args.try_dump_rsz, true, args.steamid);
    file_reader.dump_files(list)?;

    println!("Time taken: {} ms", now.elapsed().unwrap().as_millis());
    Ok(())
}
