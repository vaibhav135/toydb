use std::{collections::HashMap, env::current_dir, fs::File, io::Read};

pub mod enums;

use crate::{
    page::{OverflowHashmap, Page, PageType},
    utils::parse_be_byte_to_int,
};

#[derive(Debug)]
pub struct DBFile {
    pub db_header: Option<DBHeader>,
    pub total_pages: usize,
    pub pages: Vec<PageType>,
    pub buf: Vec<u8>,
    pub overflowpg: HashMap<usize, OverflowHashmap>,
}

#[derive(Debug)]
pub struct DBHeader {
    // I have not added all the fields (only the ones I might need for now).
    pub page_size: u16,

    // ff: file format | w: write | r: read
    pub ffw_ver: u8,
    pub ffr_ver: u8,

    pub resrv_bytes_per_pg: u8,

    pub total_freelist_pages: u32,
    pub def_pgcache_size: u32,
    pub enc_val: u32,
}

impl DBFile {
    pub fn new() -> Self {
        DBFile {
            db_header: None,
            total_pages: 0,
            pages: vec![],
            buf: vec![],
            overflowpg: HashMap::new(),
        }
    }

    pub fn init(&mut self, filename: String) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = current_dir()?;
        let filepath = format!("{}/{filename}", cwd.display());

        let mut buf: Vec<u8> = vec![];

        let total_page_size = File::open(filepath)?.read_to_end(&mut buf)?;

        self.buf = buf;

        self.read_db_header(total_page_size);

        // For 1st page first 100 bytes are db header.
        let mut start_pgheader = 100;
        for pgno in 1..=self.total_pages {
            // println!("page {pgno}");
            self.read_page(pgno, start_pgheader)?;
            start_pgheader = (self.db_header.as_ref().unwrap().page_size as usize) * pgno;
            // println!("{:?}\n\n", self.pages[pgno - 1]);
        }

        Ok(())
    }

    pub fn read_db_header(&mut self, total_page_size: usize) {
        // First 100 bytes of the 1st page is database header, and
        // that is where we are extracting page size from.
        let page_size = parse_be_byte_to_int::<u16>(&self.buf, 16);

        // Since all the pages are going to be of the same size
        // we can get the total no. of pages.
        let total_pages = total_page_size / page_size as usize;

        let resrv_bytes_per_pg = parse_be_byte_to_int(&self.buf, 20);

        let ffw_ver = parse_be_byte_to_int::<u8>(&self.buf, 18);
        let ffr_ver = parse_be_byte_to_int::<u8>(&self.buf, 19);

        let total_freelist_pages = parse_be_byte_to_int::<u32>(&self.buf, 36);
        let def_pgcache_size = parse_be_byte_to_int::<u32>(&self.buf, 48);

        // text encoding is a  4-byte BE int at offset 56 -> https://www.sqlite.org/fileformat2.html#enc
        let enc_val = parse_be_byte_to_int::<u32>(&self.buf, 56);

        self.total_pages = total_pages;

        if self.db_header.is_none() {
            self.db_header = Some(DBHeader {
                page_size,
                resrv_bytes_per_pg,
                ffw_ver,
                ffr_ver,
                total_freelist_pages,
                def_pgcache_size,
                enc_val,
            });
        }
    }
}
