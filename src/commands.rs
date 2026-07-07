use std::{fmt, str::FromStr};

use crate::file::DBFile;

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
                Commands::DbInfo => match &pages[0] {
                    crate::page::PageType::Btree(page) => {
                        if let Some(page_header) = &page.page_header {
                            println!("database page size: {}", db_header.page_size);
                            println!("number of tables: {}", page_header.num_of_cells);
                        }
                    }
                    _ => {
                        println!("Overflow page")
                    }
                },
                Commands::Tables => {}
            }
        }

        Ok(())
    }
}
