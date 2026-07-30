use std::{
    env::{self},
    process,
    str::FromStr,
};

use crate::{commands::Commands, file::Initialize};

mod btree;
mod cell;
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
    // This is temporary till we implement the shell
    if args.len() < 3 {
        return Err(Box::from("filename and command not found"));
    }

    // These are provided when we start the db (either with filename or with command)
    let filename = args.get(1).unwrap().to_string();
    let command = Commands::from_str(args.get(2).unwrap())?;

    let mut root = btree::Root::default();
    root.init(filename)?;

    print!("{:?}", root.tables);

    // Commands::process_cmd(command, &mut root)?;

    Ok(())
}
