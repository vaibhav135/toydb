use std::collections::HashMap;
use std::error::Error;
use std::{env::current_dir, fs::metadata};

pub mod enums;

use crate::btree::{DBFileInfo, DBHeader, Root, RootPage, RootPayload, SchemaType, SqlSchema};
use crate::page::Page;
use crate::utils::{parse_be_byte_to_int, read_specific_bytes};

pub trait Initialize {
    fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>>;
    fn read_db_header(&mut self, buf: &[u8]) -> DBHeader;
    fn read_root_data(
        &self,
        start_offset: u16,
        pgno: u16,
        filepath: &String,
        dbheader: &DBHeader,
        rootpg_list: &mut Vec<RootPage>,
        tables: &mut HashMap<String, SqlSchema>,
    ) -> Result<(), Box<dyn Error>>;
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

    fn read_root_data(
        &self,
        start_offset: u16,
        pgno: u16,
        filepath: &String,
        dbheader: &DBHeader,
        rootpg_list: &mut Vec<RootPage>,
        tables: &mut HashMap<String, SqlSchema>,
    ) -> Result<(), Box<dyn Error>> {
        println!("\npg no: {pgno}");

        let pgsize = dbheader.page_size;
        let pgoffset = if pgno > 0 {
            (pgno - 1) as u32 * pgsize as u32
        } else {
            start_offset as u32
        };

        let (pgheader, cells) = self.read_page(filepath, &dbheader, start_offset, pgoffset)?;

        let pgdata = self.get_pgdata(&dbheader, &pgheader, &cells)?;

        rootpg_list.push(RootPage {
            pgheader,
            pgno: pgno,
        });

        match pgdata {
            RootPayload::InteriorTable(payload) => {
                println!("interior table: {:?}", payload);
                for item in payload {
                    self.read_root_data(
                        0,
                        item.ptr as u16,
                        filepath,
                        dbheader,
                        rootpg_list,
                        tables,
                    )?;
                }
            }
            RootPayload::LeafTable(sqlschema_list) => {
                for schema in sqlschema_list {
                    tables.insert(schema.tbl_name.to_owned(), schema);
                }
            } // _ => Err(format!("Invalid root payload type...")),
        }

        Ok(())
    }

    fn read_db_header(&mut self, buf: &[u8]) -> DBHeader {
        // First 100 bytes of the 1st page is database header, and
        // that is where we are extracting page size from.
        let page_size = parse_be_byte_to_int!(buf, 16, u16);

        let resrv_bytes_per_pg = parse_be_byte_to_int!(buf, 20, u8);

        let ffw_ver = parse_be_byte_to_int!(buf, 18, u8);
        let ffr_ver = parse_be_byte_to_int!(buf, 19, u8);

        let total_freelist_pages = parse_be_byte_to_int!(&buf, 36, u32);
        let def_pgcache_size = parse_be_byte_to_int!(buf, 48, u32);

        // text encoding is a  4-byte BE int at offset 56 -> https://www.sqlite.org/fileformat2.html#enc
        let enc_val = parse_be_byte_to_int!(buf, 56, u32);

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
