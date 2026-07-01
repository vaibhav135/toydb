use std::{
    env::{self, current_dir},
    fs::{File, Metadata, metadata},
    io::Read,
    os::unix::fs::FileExt,
    process,
    str::FromStr,
};

pub mod enums;

use crate::{
    cell::{Cell, CellOperation},
    page::{Page, PageHeader},
    recordformat::RecordFormat,
    utils::{parse_be_byte_to_int, parse_varint_to_int},
};

use super::custom_error::CustomError;

use enums::BTreePageHeaderFormat;

#[derive(Debug)]
pub struct DBFile {
    pub db_header: Option<DBHeader>,
    pub total_pages: usize,
    pub pages: Vec<Page>,
    pub buf: Vec<u8>,
}

#[derive(Debug)]
pub struct DBHeader {
    // I have not added all the fields (only the ones I might need for now).
    pub page_size: u16,

    // ff: file format | w: write | r: read
    pub ffw_ver: u8,
    pub ffr_ver: u8,

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
            self.read_page(start_pgheader)?;
            start_pgheader = (self.db_header.as_ref().unwrap().page_size as usize) * pgno;
            println!("page {pgno}: {:?}\n\n", self.pages[pgno - 1]);
        }
        // println!("page 4: {:?}\n\n", self.pages[3]);

        Ok(())
    }

    pub fn read_db_header(&mut self, total_page_size: usize) {
        // First 100 bytes of the 1st page is database header, and
        // that is where we are extracting page size from.
        let page_size = parse_be_byte_to_int::<u16>(&self.buf, 16);

        // Since all the pages are going to be of the same size
        // we can get the total no. of pages.
        let total_pages = total_page_size / page_size as usize;

        let ffw_ver = parse_be_byte_to_int::<u8>(&self.buf, 18);
        let ffr_ver = parse_be_byte_to_int::<u8>(&self.buf, 19);

        let total_freelist_pages = parse_be_byte_to_int::<u32>(&self.buf, 36);
        let def_pgcache_size = parse_be_byte_to_int::<u32>(&self.buf, 48);
        let enc_val = parse_be_byte_to_int::<u32>(&self.buf, 56);

        self.total_pages = total_pages;

        if self.db_header.is_none() {
            self.db_header = Some(DBHeader {
                page_size,
                ffw_ver,
                ffr_ver,
                total_freelist_pages,
                def_pgcache_size,
                enc_val,
            });
        }
    }

    // Read page header and cell content.
    pub fn read_page(&mut self, page_start: usize) -> Result<(), Box<dyn std::error::Error>> {
        // This is the first element of the page header size: 1 byte, offset: 0
        let btree_page_type = self.get_btree_page_type(self.buf[page_start])?;

        // In B-tree page header if the btree is a interior page type we will have extra
        // 4 bytes in the end of the page header.
        let has_extra_four_bytes = match btree_page_type {
            BTreePageHeaderFormat::InteriorIndexBTreePage
            | BTreePageHeaderFormat::InteriorTableBTreePage => true,
            _ => false,
        };

        let btree_page_header_size: u8 = if has_extra_four_bytes { 12 } else { 8 };

        let total_cellptr = parse_be_byte_to_int::<u16>(&self.buf, page_start + 3);
        let mut cellarr_ptr_offset = page_start + (btree_page_header_size as usize);

        let mut cells: Vec<Cell> = vec![];

        // Each cell ptr is of 2 bytes.
        for _ in 0..total_cellptr {
            let mut cell = Cell {
                page_num_of_left_child: None,
                payload_size: None,
                rowid: None,
                payload: None,
                first_overflow_pgno: None,
            };

            let mut cur_cc_offset =
                parse_be_byte_to_int::<u16>(&self.buf, cellarr_ptr_offset) as usize;

            // From 2nd page the cell array ptr for cell content offset will be less
            // than 4096, this is to keep the size in 2bytes I guess. I will update this
            // as soon as I know the right (original) reason.
            if page_start > cur_cc_offset {
                cur_cc_offset += page_start;
            }

            if CellOperation::cell_format_validator(
                &btree_page_type,
                CellOperation::PageNumLeftChild,
            ) {
                let left_child_page_num = parse_be_byte_to_int::<u32>(&self.buf, cur_cc_offset);
                cell.page_num_of_left_child = Some(left_child_page_num);
                cur_cc_offset += 4;
            }

            let mut payload_size: u64 = 0;
            if CellOperation::cell_format_validator(
                &btree_page_type,
                CellOperation::NumOfBytesOfPayload,
            ) {
                let payload_varint_size =
                    parse_varint_to_int(&self.buf[cur_cc_offset..], &mut payload_size);
                cell.payload_size = Some(payload_size);

                cur_cc_offset += payload_varint_size;
            }

            let mut rowid: u64 = 0;
            if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Rowid) {
                let rowid_varint_size = parse_varint_to_int(&self.buf[cur_cc_offset..], &mut rowid);
                cell.rowid = Some(rowid);
                cur_cc_offset += rowid_varint_size;
            }

            if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Payload) {
                let mut record_format = RecordFormat::new();
                cur_cc_offset = record_format.set_records(&self.buf, cur_cc_offset)?;
                cell.payload = Some(record_format);
            }

            if CellOperation::cell_format_validator(
                &btree_page_type,
                CellOperation::PageNumOfFirstOverflowPage,
            ) {
                let first_overflow_page_num = parse_be_byte_to_int::<u32>(&self.buf, cur_cc_offset);
                cell.first_overflow_pgno = Some(first_overflow_page_num);
            }

            // Each cell array pointer is of 2 bytes
            cellarr_ptr_offset += 2;

            cells.push(cell);
        }

        let first_freeblock_start = parse_be_byte_to_int::<u16>(&self.buf, page_start + 1);
        let cell_content_area = parse_be_byte_to_int::<u16>(&self.buf, page_start + 5);
        let fragmented_cellcontent_area = parse_be_byte_to_int::<u8>(&self.buf, page_start + 6);

        let rightmost_ptr = if has_extra_four_bytes {
            Some(parse_be_byte_to_int::<u32>(&self.buf, page_start + 7))
        } else {
            None
        };

        let page_header = PageHeader {
            btree_page_type,
            first_freeblock_start,
            num_of_cells: total_cellptr,

            start_ccarea: cell_content_area,
            frag_ccarea: fragmented_cellcontent_area,

            rightmost_ptr,
        };

        self.pages.push(Page {
            page_header: Some(page_header),
            cells,
        });

        Ok(())
    }

    // pub fn get_dbheader(&self) -> &Option<DBHeader> {
    //     &self.db_header
    // }
    //
    // pub fn get_pages(&self) -> &Vec<Page> {
    //     &self.pages
    // }

    // In the page header format table it's mentioned, for which offset
    // what is the type of b-tree page we have.
    pub fn get_btree_page_type(
        &self,
        offset_val: u8,
    ) -> Result<BTreePageHeaderFormat, CustomError> {
        match offset_val {
            2 => Ok(BTreePageHeaderFormat::InteriorIndexBTreePage),
            5 => Ok(BTreePageHeaderFormat::InteriorTableBTreePage),
            10 => Ok(BTreePageHeaderFormat::LeafIndexBTreePage),
            13 => Ok(BTreePageHeaderFormat::LeafTableBTreePage),
            _ => Err(CustomError::InvalidOffsetValueError),
        }
    }
}
