use std::{env::current_dir, fs::metadata};

pub mod enums;

use crate::btree::{DBFileInfo, DBHeader, Root};
use crate::utils::{parse_be_byte_to_int, read_specific_bytes};

pub trait Initialize {
    fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>>;
    fn read_db_header(&mut self, buf: &[u8]) -> DBHeader;
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
