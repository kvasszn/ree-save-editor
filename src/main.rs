use std::{io::Cursor, path::PathBuf};

use clap::{Parser, Subcommand};
use ree_lib::rsz::RszMap;
use ree_save_core::{game_context::{GameConfigs, load_game_configs}, save::{SaveFile, SaveFlags, SaveOptions, corrupt::CorruptSaveReader, game::Game}};
use anyhow::{anyhow, Context};

#[derive(Parser, Debug)]
#[command(name = "ree-save-cli")]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(long, help = "ID to use for the save file")]
    id: Option<u64>,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(long)]
    overwrite: bool,
    #[arg(short, long)]
    game: Game,
    #[arg(short, long)]
    brute_force: bool,
    #[arg(long)]
    bf_start: Option<u64>,
    #[arg(long)]
    bf_count: Option<u64>,
    #[arg(long)]
    dump_raw: bool,
    #[command(subcommand)]
    command: Command
}

fn parse_hex(s: &str) -> Result<u32, std::num::ParseIntError> {
    let clean_str = s.trim_start_matches("0x");
    u32::from_str_radix(clean_str, 16)
}

#[derive(Debug, Subcommand)]
enum Command {
    Convert {
        #[arg(long)]
        id: Option<u64>,
        #[arg(long)]
        citrus: Option<usize>,
        #[arg(long)]
        game: Option<Game>,
        #[arg(long)]
        flags: Option<SaveFlags>
    },
    ToPS5,
    DumpBytes,
    DumpJson,
    CorruptFix {
        #[arg(long)]
        classes: Vec<String>,
        #[arg(long, value_parser = parse_hex)]
        typeids: Vec<u32>,
        #[arg(long, default_value_t=String::from("UserSaveData"))]
        schema: String
    }
    // TODO: when i add lua again, add running lua scripts from cli
}

pub fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,egui=info")
    ).init();

    let cli = Cli::parse();
    println!("{:?}", cli);

    let save_file_data = std::fs::read(&cli.input)?;

    let mut read_opts = SaveOptions::new(cli.game);
    read_opts.id = cli.id;
    read_opts.dump = cli.dump_raw;

    if cli.brute_force {
        read_opts = read_opts.brute_force(0x0110000100000000, 0xffffffff);
    }

    if let Some(start) = cli.bf_start &&
        let Some((s, _)) = read_opts.brute_force.as_mut() {
        *s = start;
    }

    if let Some(count) = cli.bf_count &&
        let Some((_, c)) = read_opts.brute_force.as_mut() {
        *c = count;
    }

    let mut output_file = cli.output.unwrap_or_else(|| cli.input.clone());
    if output_file.is_dir() {
        if let Some(file_name) = cli.input.file_name() {
            output_file.push(file_name);
        } else {
            return Err(anyhow!("Input path does not have a valid filename to use for the directory output"));
        }
    }

    if output_file.exists() {
        if cli.overwrite {
            log::warn!("overwrite is being used and {} already exists", output_file.display());
        } else {
            return Err(anyhow!("{} already exists. Use the --overwrite option to overwrite it", output_file.display()));
        }
    }

    if let Some(parent_dir) = output_file.parent() {
        std::fs::create_dir_all(parent_dir).with_context(|| {
            format!(
                "Failed to create output directory structure at: {}",
                parent_dir.display()
            )
        })?;
    }

    match cli.command {
        // TODO: add presets for Converting? i.e to/from PS5 for each game
        Command::Convert { id, citrus, game, flags } => {
            let mut save_file = match SaveFile::read_save(save_file_data, &mut read_opts) {
                Ok(s) => s,
                Err(e) => return Err(anyhow!("Could not read save at {}: {e}", cli.input.display())),
            };

            if read_opts.brute_force.is_some() && let Some(id) = read_opts.id {
                log::info!("Brute forced ID={}", id);
            }

            if let Some(flags) = flags {
                save_file.flags = flags;
            }
            let mut write_opts = SaveOptions::new(game.unwrap_or(cli.game));
            write_opts.id = id;
            write_opts.curve_index = citrus;

            log::info!("Writing save file to: {}", output_file.display());

            // TODO: make this return Err out of main
            let res = save_file.save(&output_file, &write_opts);
            if let Err(e) = res {
                log::error!("Error writing save: {e}");
            }
        },
        Command::DumpBytes => {
            let mut save_file = match SaveFile::read_save(save_file_data, &mut read_opts) {
                Ok(s) => s,
                Err(e) => return Err(anyhow!("Could not read save at {}: {e}", cli.input.display())),
            };

            if read_opts.brute_force.is_some() && let Some(id) = read_opts.id {
                log::info!("Brute forced ID={}", id);
            }

            save_file.flags = SaveFlags::empty();
            let write_opts = SaveOptions::new(cli.game);
            log::info!("Dumping save bytes to: {}", output_file.display());
            let res = save_file.save(&output_file, &write_opts);
            if let Err(e) = res {
                log::error!("Error writing save: {e}");
            }
        }
        Command::CorruptFix { classes, typeids, schema } => {
            if read_opts.brute_force.is_some() && let Some(id) = read_opts.id {
                log::info!("Brute forced ID={}", id);
            }

            let game_configs = load_game_configs("game_configs.json")?;
            let Some(game_config) = &game_configs.get(&cli.game) else {
                return Err(anyhow!("Game Config for {:?} does not exist", cli.game))
            };
            let Some(rsz_path) = &game_config.rsz else {
                return Err(anyhow!("Game Config for {:?} does not have rsz", cli.game))
            };
            let rsz = std::fs::read(rsz_path)?;
            let rsz: RszMap = serde_json::from_slice(&rsz)?;

            let mut corrupt_reader = CorruptSaveReader::new(&rsz, cli.game);
            let (raw_data, offset, _, flags) = match SaveFile::process_bytes_to_stream(save_file_data, &mut read_opts) {
                Ok(s) => s,
                Err(e) => return Err(anyhow!("Could not read save at {}: {e}", cli.input.display())),
            };
            let mut data = Cursor::new(&raw_data[offset as usize..]);

            let types: Vec<(u32, &str)> = if !classes.is_empty() && !typeids.is_empty() {
                if classes.len() == typeids.len() {
                    typeids.iter().zip(&classes).map(|x| (*x.0, x.1.as_ref())).collect()
                } else {
                    return Err(anyhow!("Classes and typeids must be the same length"));
                }
            } else {
                let Some(schema) = game_config.schemas.get(&schema) else {
                    return Err(anyhow!("Could not find schema with name {schema} for game {:?}", cli.game));
                };
                schema.iter().map(|x| (x.0, x.1.as_ref())).collect()
            };

            let mut save_file = corrupt_reader.read_missing(&mut data, &types);
            save_file.flags = flags;

            let mut write_opts = SaveOptions::new(cli.game);
            write_opts.id = read_opts.id;
            let res = save_file.save(&output_file, &write_opts);
            if let Err(e) = res {
                log::error!("Error writing save: {e}");
            }
        }
        _ => {}
    }
    Ok(())
}
