use std::{error::Error, fmt, str::FromStr};

use crate::btree::{Root, SchemaType};

#[derive(Debug)]
pub enum Commands {
    DbInfo,
    Tables,
}

impl FromStr for Commands {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ".dbinfo" => Ok(Commands::DbInfo),
            ".tables" => Ok(Commands::Tables),
            _ => Err(format!("Unknown command!!!")),
        }
    }
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Commands::DbInfo => write!(f, ".dbinfo"),
            Commands::Tables => write!(f, ".tables"),
        }
    }
}

impl Commands {
    pub fn is_valid(cmd: &str) -> bool {
        if self::Commands::from_str(cmd).is_err() {
            return false;
        }
        true
    }

    pub fn process_cmd(cmd: &str, root: &Root) -> Result<(), Box<dyn Error>> {
        let command = self::Commands::from_str(cmd)
            .expect("We have already validate it previously so no need to return the error");

        let db_header = &root.db_header;
        let tables = &root.tables;

        let pg_size = db_header.page_size;
        let num_of_tables = tables.len();

        match command {
            Commands::DbInfo => {
                println!("database page size: {pg_size}");
                println!("number of tables: {num_of_tables}");
            }
            Commands::Tables => {
                tables
                    .values()
                    .flatten()
                    .filter(|schema| schema.schema_type == SchemaType::TABLE)
                    .for_each(|schema| println!("{}", schema.tbl_name));
            }
        }

        Ok(())
    }
}
