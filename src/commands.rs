use std::{fmt, str::FromStr};

use crate::{file::DBFile, page::PageType, recordformat::RecordDataType};

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
                Commands::Tables => {
                    let PageType::Btree(schemapg) = &dbfile.pages[0] else {
                        return Err("Sqlite schema page not found!!!".into());
                    };

                    for cell in &schemapg.cells {
                        if let Some(payload) = &cell.payload {
                            let rowsize = payload.rows.len();
                            let rows = &payload.rows;

                            let mut rowidx = 0;
                            // We are ignoring the the term table, the schema name last content cause that is just a CREATE query
                            // usually.
                            while rowidx < rowsize - 1 {
                                if let RecordDataType::STR(data) = &rows[rowidx].content {
                                    if data.to_lowercase() == "table" {
                                        rowidx += 2;
                                    } else if data.to_lowercase().starts_with("sqlite_") {
                                        break;
                                    } else {
                                        println!("{data}");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
