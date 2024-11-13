//! # synchronizer
//!
//! > We impose order on the chaos of organic evolution. You exist because we allow it. And you will end because we demand it.
//! >
//! > -- Sovereign, Mass Effect
//!
//! AT-TPC data is messy. Running analysis on messy data is hard.
//!
//! The synchronizer is an effort to impose order on the chaos of runs in real data. It takes a set of AT-TPC runs and synchronize the data bits according to their time stamps.
//!
//! ## Installation
//!
//! The synchronizer is written in Rust and requires a Rust compiler. The Rust toolchain can be installed from [here](https://rust-lang.org).
//!
//! Once the Rust toolchain is installed, download the repository from GitHub
//!
//! ```bash
//! git clone https://github.com/ATTPC/synchronizer.git
//! ```
//!
//! From within the repository run
//!
//! ```bash
//! cargo install
//! ```
//!
//! This will install the synchronizer executable to your cargo installs and it will be available on your path as `synchronizer`.
//!
//! ## Use
//!
//! The synchronizer uses the following CLI:
//!
//! ```txt
//! synchronizer --config/-c /path/to/some/config.yml
//! ```
//!
//! where `/path/to/some/config.yml` should be replaced with the path to an appropriate configuration file. The synchronizer can generate a default template for you using the `new` command.
//!
//! ```txt
//! synchronizer --config/-c /path/to/some/config.yml new
//! ```
//!
//! ### Configuration
//!
//! Configurations are defined as the following YAML:
//!
//! ```yaml
//! merger_path: "/path/to/some/merger/data/"
//! sync_path: "/path/to/some/synchronic/data/"
//! min_run: 55
//! max_run: 69
//! ```
//!
//! Some important notes:
//!
//! - The path given as the `sync_path` must exist before running the synchronizer
//! - Min run and max run are the range of run numbers (*merger run numbers*) to be synchronized. The range is inclusive; run numbers can be missing in the range.
//!
//! ### Output Format
//!
//! The synchronizer follows the current [attpc_merger](https://github.com/attpc_merger) format, with some minor changes. That format is
//!
//! ```txt
//! run_0001.h5
//! |---- events - min_event, max_event, version
//! |    |---- event_# - orig_run, orig_event
//! |    |    |---- get_traces(dset) - id, timestamp, timestamp_other
//! |    |    |---- frib_physics - id, timestamp
//! |    |    |    |---- 977(dset)
//! |    |    |    |---- 1903(dset)
//! ```
//!
//! ## Why would you do this to me?
//!
//! Because due to some gremlins roaming around the hardware during the experiment, the GET and FRIB DAQs didn't have the same busy and/or trigger!
//! Or something was very wrong with the VMUSB!
mod config;
mod reader;
mod scalers;
mod writer;

use clap::{Arg, Command};
use color_eyre::eyre::Result;
use config::Config;
// use human_bytes::human_bytes;
// use indicatif::{ProgressBar, ProgressStyle};
// use reader::{get_total_merger_bytes, get_total_merger_events, MergerReader};
use reader::{MergerReader};
// use scalers::process_scalers;
use std::path::PathBuf;
use writer::SyncWriter;
use scalers::{copy_scalers_010, copy_scalers_020};
use crate::reader::construct_run_path;

/// Main processing loop. Takes the config and synchronizes the data for each run.
pub fn synchronize(config: Config) -> Result<()> {
    for run in config.min_run..=config.max_run {
        let path = construct_run_path(&config.merger_path, run);
        if !path.exists() {
            println!("Run {} doesn't exist, skipping...", run);
            continue;
        }
        let mut reader = MergerReader::new(&config.merger_path, run)?;
        println!("Processing run {}...", &run);
        let mut writer = SyncWriter::new(&config.sync_path, run)?;
        // First read all data and create TS lists
        println!("Reading time stamps...");
        reader.read_ts()?;
        // Synchronize TS
        println!("Synchronizing time stamps...");
        reader.sync_ts();
        // Write synchronized run using lists
        println!("Writing synchronized file...");
        for i in 0..reader.get_sync.len() {
            reader.current_event = reader.get_sync[i] as u64;
            let get_event = reader.read_event()?;
            reader.current_event = reader.frib_sync[i] as u64;
            let frib_event = reader.read_event()?;
            writer.write_combined(get_event.expect("GET event not found"), frib_event.expect("FRIB event not found"))?;
        }
        // Process scalers
        let parent_groups = reader.current_file.member_names()?;
        if parent_groups.contains(&String::from("meta")) {
            copy_scalers_010(&reader, &writer)?;
        } else if parent_groups.contains(&String::from("events")) {
            copy_scalers_020(&reader, &writer)?;
        }

        // Close file
        writer.close()?;
    }
    Ok(())
}

/// Program entry point. Handles the CLI.
fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Command::new("synchronizer")
        .arg_required_else_help(true)
        .subcommand(Command::new("new").about("Create a new template config file"))
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("Path to a configuration file (YAML)"),
        )
        .get_matches();

    println!("--------------------- AT-TPC Synchronizer ---------------------");
    let config_path = PathBuf::from(cli.get_one::<String>("config").expect("We require args"));

    // Handle the new subcommand
    if let Some(("new", _)) = cli.subcommand() {
        println!(
            "Making a template configuration file at {}...",
            config_path.display()
        );
        Config::default().save(&config_path)?;
        println!("Done.");
        println!("-------------------------------------------------------------");
        return Ok(());
    }

    let config = Config::load(&config_path)?;
    println!(
        "Successfully loaded configuration from {}",
        config_path.display()
    );

    if !config.merger_path.exists() {
        println!(
            "Merger path {} does not exist! Quitting.",
            config.merger_path.display()
        );
        println!("-------------------------------------------------------------");
    } else if !config.sync_path.exists() {
        println!(
            "Synchronized path {} does not exist! Please create it before running the synchronizer.",
            config.sync_path.display()
        );
        println!("-------------------------------------------------------------");
    }

    println!("Synchronizing...");
    synchronize(config)?;
    println!("Complete.");

    println!("-------------------------------------------------------------");

    Ok(())
}
