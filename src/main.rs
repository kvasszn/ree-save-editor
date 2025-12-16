pub mod gensdk;
pub mod reerr;
pub mod bitfield;
pub mod compression;
pub mod rsz;
pub mod file;
pub mod save;

#[cfg(feature = "tdb")]
pub mod tdb;
pub mod edit;

extern crate image;

use clap::Parser;
use file::FileReader;
use mhtame::file::User;
use mhtame::rsz::dump::RszDump;
use rsz::dump::{ENUM_FILE, RSZ_FILE};
use sdk::deserializer::RszDeserializer;
use sdk::json_serializer::RszWithCtx;
use sdk::type_map::TypeMap;

use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::{BufWriter, Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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
    
    #[arg(long, default_value_t = String::from("assets/rszmhwilds.json"))]
    rsz: String,
    
    #[arg(long, default_value_t = String::from("assets/enumsmhwilds.json"))]
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

    /*let rsz_file = File::open(&args.rsz)?;
    let mut enums_file = File::open(&args.enums)?;
    let mut data = Vec::new();
    enums_file.read_to_end(&mut data)?;
    let enums_file = File::open(&args.enums)?;
    let type_map = TypeMap::from_reader(rsz_file, enums_file)?;*/
    /*println!("Took {time_taken}ms to load map");
    type_map.to_bincode("rszmhwilds.bin")?;*/

    /*println!("loading from bincode");
    let now = SystemTime::now();
    let data = std::fs::read("./assets/types.bin")?;
    let type_map = TypeMap::parse_bincode(&data)?;
    let time_taken = now.elapsed().unwrap().as_millis();
    println!("Took {time_taken}ms to load map");

    let _ = RszDump::get_struct(0);

    if let Some(file) = &args.file_name {
        let now = SystemTime::now();
        let f = File::open(file)?;
        let user = User::new(f)?;
        let rsz = user.rsz;
        let data = Cursor::new(rsz.data);
        let hash_map = HashMap::new();
        let mut rsz_deserializer = RszDeserializer::new(data, &rsz.roots, &rsz.type_descriptors, &type_map, &hash_map);
        let rsz = rsz_deserializer.deserialize()?;
        let rsz_with_ctx = RszWithCtx {
            rsz: &rsz,
            type_map: &type_map
        };
        let file = File::create("outputs/tests/enum_rsz.json")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &rsz_with_ctx)?;
        //println!("{:#?}", rsz);
        println!("Enum Rsz took {}ms", now.elapsed()?.as_millis());
    }
    if let Some(file) = &args.file_name {
        let now = SystemTime::now();
        let f = File::open(file)?;
        let user = User::new(f)?;
        let dersz = user.rsz.deserialize_to_dersz()?;
        let file = File::create("outputs/tests/shit_rsz.json")?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &dersz)?;
        //println!("{:#?}", rsz);
        println!("Dyn Rsz took {}ms", now.elapsed()?.as_millis());
    }*/


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
