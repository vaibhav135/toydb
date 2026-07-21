use std::{collections::HashMap, env::current_dir, fs::metadata};

pub mod enums;

use crate::utils::{parse_be_byte_to_int, read_specific_bytes};

#[derive(Debug, Default)]
pub struct DBFileInfo {
    total_dbsize: usize,
    filepath: String,
}

// Root is the first page.
#[derive(Debug, Default)]
pub struct Root {
    pub db_header: DBHeader,
    pub total_pages: usize,

    // Root usually have tables either interior or leaf.
    pub tables: HashMap<SchemaType, Vec<String>>,

    pub metadata: DBFileInfo,
    // pub pages: Vec<PageType>,
    // pub buf: Vec<u8>,
    // pub overflowpg: HashMap<usize, OverflowHashmap>,
}

#[derive(Debug, Default)]
pub struct DBHeader {
    // I have not added all the fields (only the ones I might need for now).
    pub page_size: u16,

    // ff: file format | w: write | r: read
    pub ffw_ver: u8,
    pub ffr_ver: u8,

    pub resrv_bytes_per_pg: u8,

    pub total_freelist_pages: u32,

    // def: default
    pub def_pgcache_size: u32,
    pub enc_val: u32,
}

#[derive(Debug)]
pub struct SqlSchema {
    schema_type: SchemaType, // could be a table, index, view or trigger
    name: String,            // name of the object
    tbl_name: String,        // name of table or view the object is associated with
    rootpg: u32,
    sql: String,
}

#[derive(Debug, PartialEq)]
pub enum SchemaType {
    TABLE,
    INDEX,
    VIEW,
    TRIGGER,
}

impl TryFrom<String> for SchemaType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let match_value = value.as_str();
        match match_value {
            "table" => Ok(SchemaType::TABLE),
            "index" => Ok(SchemaType::INDEX),
            "trigger" => Ok(SchemaType::TRIGGER),
            "view" => Ok(SchemaType::VIEW),
            _ => Err(format!("invalid schema type!!!")),
        }
    }
}

#[derive(Debug)]
pub struct InteriorTablePayload {
    ptr: u32,
    key: u64,
}

#[derive(Debug)]
pub enum RootPayload {
    InteriorTable(InteriorTablePayload),
    LeafTable(SchemaType),
}

pub trait Initialize {
    fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>>;
    fn read_db_header(&mut self, buf: &[u8]) -> DBHeader;
}

impl Initialize for Root {
    fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        // let abc: SchemaType = String::try_from("xyz")?.try_into()?;
        //
        // println!("{:?}", abc);

        let cwd = current_dir()?;
        let filepath = format!("{}/{filename}", cwd.display());

        let total_page_size = metadata(&filepath).unwrap().len() as usize;

        let fileinfo = DBFileInfo {
            total_dbsize: total_page_size,
            filepath: filepath,
        };

        let dbheader_bytes = read_specific_bytes(&fileinfo.filepath, 0, 99)?;
        let db_header = self.read_db_header(&dbheader_bytes);

        // Since all the pages are going to be of the same size
        // we can get the total no. of pages.
        self.total_pages = total_page_size / db_header.page_size as usize;

        // self.read_sql_shcema();

        // For 1st page first 100 bytes are db header.
        // let mut start_pgheader = 100;
        // for pgno in 1..=self.total_pages {
        //     println!("page {pgno}");
        //     self.read_page(pgno, start_pgheader)?;
        //     start_pgheader = (self.db_header.as_ref().unwrap().page_size as usize) * pgno;
        //     println!("{:?}\n\n", self.pages[pgno - 1]);
        // }

        Ok(())
    }

    fn read_db_header(&mut self, buf: &[u8]) -> DBHeader {
        // First 100 bytes of the 1st page is database header, and
        // that is where we are extracting page size from.
        let page_size = parse_be_byte_to_int::<u16>(buf, 16);

        let resrv_bytes_per_pg = parse_be_byte_to_int(buf, 20);

        let ffw_ver = parse_be_byte_to_int::<u8>(buf, 18);
        let ffr_ver = parse_be_byte_to_int::<u8>(buf, 19);

        let total_freelist_pages = parse_be_byte_to_int::<u32>(&buf, 36);
        let def_pgcache_size = parse_be_byte_to_int::<u32>(buf, 48);

        // text encoding is a  4-byte BE int at offset 56 -> https://www.sqlite.org/fileformat2.html#enc
        let enc_val = parse_be_byte_to_int::<u32>(buf, 56);

        // self.total_pages = total_pages;

        DBHeader {
            page_size,
            resrv_bytes_per_pg,
            ffw_ver,
            ffr_ver,
            total_freelist_pages,
            def_pgcache_size,
            enc_val,
        }
    }
}
