use super::file::enums::BTreePageHeaderFormat;
use crate::{
    cell::CellOperation,
    file::{DBFile, DBHeader},
    recordformat::RecordFormat,
    utils::{parse_be_byte_to_int, parse_varint_to_int},
};

use std::{cell, fmt, io::Bytes, str::FromStr};

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
    pub fn process_cmd(
        command: Commands,
        dbfile: &mut DBFile,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // let db_header = &dbfile.db_header;

        if let Some(db_header) = &dbfile.db_header {
            let pages = &dbfile.pages;
            match command {
                Commands::DbInfo => {
                    if let Some(page_header) = &pages[0].page_header {
                        println!("database page size: {}", db_header.page_size);
                        println!("number of tables: {}", page_header.num_of_cells);
                    }
                }
                Commands::Tables => {

                    /*
                     *   U : usable size of a database page
                     *       usable page = Total page size - reserved space at the end of each page
                     *
                     *   P: payload size
                     *
                     *   X: maximum amount of payload that be stored directly on the b-tree page before
                     *      spilling onto an overflow page
                     *
                     *   M : minimum amount of payload that must be stored on the btree page before spilling
                     *       allowed
                     *
                     */
                }
            }
        }

        Ok(())
    }
}
