use std::{
    env::{self, current_dir},
    fmt,
    fs::File,
    io::Read,
    process,
    str::FromStr,
};

fn parse_byte_to_u16(buf: &Vec<u8>, start_byte: usize, end_byte: usize) -> u16 {
    let byte_slice: [u8; 2] = buf[start_byte..end_byte].try_into().unwrap();

    u16::from_be_bytes(byte_slice)
}

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

            let mut buf = Vec::new();
            let _total_page_size = File::open(filepath)?.read_to_end(&mut buf)?;

            let page_size = parse_byte_to_u16(&buf, 16, 18);
            let cell_count = parse_byte_to_u16(&buf, 103, 105);
            println!("database page size: {}", page_size);
            println!("number of tables: {}", cell_count);
        }
    }

    Ok(())
}
