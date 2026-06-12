use std::{
    env::{self, current_dir},
    fmt,
    fs::File,
    io::Read,
    process,
    str::FromStr,
};

#[derive(Debug)]
enum Commands {
    DbInfo,
}

impl FromStr for Commands {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ".dbinfo" => Ok(Commands::DbInfo),
            _ => Err(format!("Unknown command!!!")),
        }
    }
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Commands::DbInfo => write!(f, ".dbinfo"),
        }
    }
}

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

    match command.parse()? {
        Commands::DbInfo => {
            let cwd = current_dir().unwrap();
            let filepath = format!("{}/{filename}", cwd.display());
            let mut buf = [0; 100];
            File::open(filepath)?.read(&mut buf[..])?;

            let byte_slice: [u8; 2] = buf[16..18].try_into().unwrap();

            let page_size = u16::from_be_bytes(byte_slice);
            println!("database page size: {}", page_size);
        }
    }

    Ok(())
}
