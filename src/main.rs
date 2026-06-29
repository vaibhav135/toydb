use std::{
    env::{self, current_dir},
    fs::File,
    io::Read,
    process,
};

mod cell;
mod commands;
mod custom_error;
mod fileformat;
mod recordformat;
mod utils;

use commands::process_cmd;

fn main() {
    let res = run();
    if res.is_err() {
        eprintln!("{}", res.unwrap_err());
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        return Err(Box::from("filename and command not found"));
    }

    let filename = &args[1];
    let command = &args[2];

    let cwd = current_dir()?;
    let filepath = format!("{}/{filename}", cwd.display());

    let mut buf: Vec<u8> = Vec::new();
    let total_page_size = File::open(filepath)?.read_to_end(&mut buf)?;

    process_cmd(command, buf.as_slice(), total_page_size)?;

    Ok(())
}
