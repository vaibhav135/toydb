use std::{
    env::{self},
    process,
    str::FromStr,
};

use crate::{cli::Cli, commands::Commands, file::Initialize};

mod btree;
mod cell;
mod cli;
mod commands;
mod custom_error;
mod file;
mod page;
mod schema;
mod utils;

fn main() {
    let res = run();
    if res.is_err() {
        eprintln!("{}", res.unwrap_err());
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();

    // These are provided when we start the db (either with filename or with command)
    let filename: Option<String> = args.get(1).map(|s| s.to_string());
    let cmd = args.get(2).map(|s| s.to_string());

    let cli = Cli { filename, cmd };

    cli.init()?;

    Ok(())
}
