use std::collections::HashMap;
use std::{env::current_dir, fs::metadata};

pub mod enums;

use crate::btree::{DBFileInfo, Root, RootPage, SqlSchema};
use crate::utils::read_specific_bytes;

pub trait Initialize {
    fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>>;
}

impl Initialize for Root {
    fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = current_dir()?;
        let filepath = format!("{}/{filename}", cwd.display());

        let total_page_size = metadata(&filepath).unwrap().len() as usize;

        let fileinfo = DBFileInfo {
            total_dbsize: total_page_size,
            filepath: filepath,
        };

        let dbheader_bytes = read_specific_bytes(&fileinfo.filepath, 0, 99)?;
        let dbheader = self.read_db_header(&dbheader_bytes);

        let mut tables: HashMap<String, SqlSchema> = HashMap::new();

        let mut rootpg_list: Vec<RootPage> = vec![];

        self.read_root_data(
            100,
            0,
            &fileinfo.filepath,
            &dbheader,
            &mut rootpg_list,
            &mut tables,
        )?;

        // Since all the pages are going to be of the same size
        // we can get the total no. of pages.
        self.total_pages = total_page_size / dbheader.page_size as usize;
        self.metadata = fileinfo;
        self.db_header = dbheader;
        self.tables = tables;
        self.pages = rootpg_list;

        Ok(())
    }
}
