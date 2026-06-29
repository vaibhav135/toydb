use super::fileformat::{enums::BTreePageHeaderFormat, get_btree_page_type};
use crate::{
    cell::CellOperation,
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

pub fn process_cmd(
    command: &String,
    buf: &[u8],
    total_page_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // First 100 bytes of the 1st page is database header, and
    // that is where we are extracting page size from.
    let page_size = parse_be_byte_to_int::<u16>(buf, 16);

    // Since all the pages are going to be of the same size
    // we can get the total no. of pages.
    let total_pages = total_page_size / page_size as usize;

    match command.parse()? {
        Commands::DbInfo => {
            let cell_count = parse_be_byte_to_int::<u16>(buf, 103);
            println!("database page size: {}", page_size);
            println!("number of tables: {}", cell_count);
        }
        Commands::Tables => {
            // This is the first element of the page header size: 1 byte, offset: 0
            let btree_page_type_offset_val = &buf[100];
            let btree_page_type = get_btree_page_type(*btree_page_type_offset_val)?;
            println!("B-Tree page type: {:?}", btree_page_type);

            let has_both_interior_pages = match btree_page_type {
                BTreePageHeaderFormat::InteriorIndexBTreePage
                | BTreePageHeaderFormat::InteriorTableBTreePage => true,
                _ => false,
            };

            // In B-tree page header if the btree is a interior page type we will have extra
            // 4 bytes in the end of the page header.
            let has_extra_four_bytes = match btree_page_type {
                BTreePageHeaderFormat::InteriorIndexBTreePage
                | BTreePageHeaderFormat::InteriorTableBTreePage => true,
                _ => false,
            };

            let btree_page_header_size: u8 = if has_extra_four_bytes { 12 } else { 8 };

            let total_cellptr = parse_be_byte_to_int::<u16>(buf, 103);
            let mut cellptr_offset = 100 + btree_page_header_size;
            // let mut cell_content_offsets = Vec::with_capacity(total_cellptr as usize);

            // Each cell ptr is of 2 bytes.
            for idx in 0..total_cellptr {
                let cell_pointer_offset = parse_be_byte_to_int::<u16>(buf, cellptr_offset as usize);
                // cell_content_offsets.push(point_to_offset);
                println!("cell no. {idx} cell value: {cell_pointer_offset}");
                cellptr_offset += 2;

                let mut current_cell_offset = cell_pointer_offset as usize;

                let mut left_child_page_num = 0;
                if CellOperation::cell_format_validator(
                    &btree_page_type,
                    CellOperation::PageNumLeftChild,
                ) {
                    left_child_page_num = parse_be_byte_to_int::<u32>(buf, current_cell_offset);
                    current_cell_offset += 4;
                }

                let mut payload_size: u64 = 0;
                if CellOperation::cell_format_validator(
                    &btree_page_type,
                    CellOperation::NumOfBytesOfPayload,
                ) {
                    let payload_varint_size =
                        parse_varint_to_int(&buf[current_cell_offset..], &mut payload_size);

                    current_cell_offset += payload_varint_size;
                    // println!("payload size: {payload_size}");
                    // println!("payload varint size: {payload_varint_size}");
                }

                let mut rowid: u64 = 0;
                if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Rowid) {
                    let rowid_varint_size =
                        parse_varint_to_int(&buf[current_cell_offset..], &mut rowid);
                    current_cell_offset += rowid_varint_size;
                    // println!("rowid : {rowid}");
                    // println!("rowid varint size: {rowid_varint_size}");
                }

                if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Payload) {
                    let mut record_format = RecordFormat::new();
                    current_cell_offset = record_format.set_records(buf, current_cell_offset)?;

                    println!("{:?}", record_format);
                }

                if CellOperation::cell_format_validator(
                    &btree_page_type,
                    CellOperation::PageNumOfFirstOverflowPage,
                ) {}
            }

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

    Ok(())
}
