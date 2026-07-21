use std::collections::HashMap;

use crate::{
    file::{DBHeader, enums::BTreePageHeaderFormat},
    page::{Page, PageHeader},
    recordformat::RecordFormat,
    utils::{parse_be_byte_to_int, parse_varint_to_int},
};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum CellOperation {
    PageNumLeftChild,
    NumOfBytesOfPayload,
    Rowid,
    Payload,
    PageNumOfFirstOverflowPage,
}

#[derive(Debug)]
pub struct Cell {
    pub pgnum_of_left_child: Option<u32>,
    pub payload_size: u64,
    pub rowid: Option<u64>,
    pub payload: Option<Vec<u8>>,
    pub first_overflow_pgno: u32,
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            pgnum_of_left_child: None,
            payload_size: 0,
            rowid: None,
            payload: None,
            first_overflow_pgno: 0,
        }
    }

    pub fn read_cell(
        &mut self,
        pgheader: &PageHeader,
        dbheader: &DBHeader,
        start_byte: usize,
        buf: &[u8],
    ) {
        let mut contentcell_offset = start_byte;
        let cell_field_map = CellOperation::get_valid_cell_fields(&pgheader.btree_pgtype);

        if cell_field_map
            .get(&CellOperation::PageNumLeftChild)
            .is_some()
        {
            self.pgnum_of_left_child = Some(parse_be_byte_to_int::<u32>(buf, contentcell_offset));

            contentcell_offset += 4;
        }

        if cell_field_map
            .get(&CellOperation::NumOfBytesOfPayload)
            .is_some()
        {
            let payload_varint_size =
                parse_varint_to_int(&buf[contentcell_offset..], &mut self.payload_size);

            contentcell_offset += payload_varint_size;
        }

        if cell_field_map.get(&CellOperation::Rowid).is_some() {
            let mut rowid = 0;
            let rowid_varint_size = parse_varint_to_int(&buf[contentcell_offset..], &mut rowid);
            self.rowid = Some(rowid);

            contentcell_offset += rowid_varint_size;
        }

        if cell_field_map.get(&CellOperation::Payload).is_some() {
            let payload_overflow_bytes = CellOperation::get_payload_overflow_bytes(
                &(dbheader.page_size as usize),
                dbheader.resrv_bytes_per_pg,
                self.payload_size as usize,
                &pgheader.btree_pgtype,
            );

            let current_cc_size = self.payload_size as usize - payload_overflow_bytes;

            self.payload =
                Some(buf[contentcell_offset..contentcell_offset + current_cc_size].to_vec());

            contentcell_offset += current_cc_size;
        }

        if cell_field_map
            .get(&CellOperation::PageNumOfFirstOverflowPage)
            .is_some()
        {
            self.first_overflow_pgno = parse_be_byte_to_int::<u32>(buf, contentcell_offset);
        }
    }

    pub fn get_cellptr_arr(num_of_cells: u16, buf: &[u8]) -> Vec<u16> {
        let mut cellptr_arr = vec![];

        // Each cell ptr is 2 byte
        for idx in 0..num_of_cells {
            let cellptr = parse_be_byte_to_int::<u16>(buf, idx as usize);
            cellptr_arr.push(cellptr);
        }

        cellptr_arr
    }
}

impl CellOperation {
    const CELL_FIELDS: [CellOperation; 5] = [
        CellOperation::NumOfBytesOfPayload,
        CellOperation::Rowid,
        CellOperation::PageNumLeftChild,
        CellOperation::PageNumOfFirstOverflowPage,
        CellOperation::Payload,
    ];

    pub fn get_valid_cell_fields(
        btree_type: &BTreePageHeaderFormat,
    ) -> HashMap<&CellOperation, bool> {
        let mut cell_field_map = HashMap::new();

        for field in CellOperation::CELL_FIELDS.iter() {
            // Check the bcell format in here -> https://www.sqlite.org/fileformat.html#the_database_header
            let is_valid = match field {
                CellOperation::PageNumLeftChild => match btree_type {
                    BTreePageHeaderFormat::InteriorIndexBTreePage
                    | BTreePageHeaderFormat::InteriorTableBTreePage => true,
                    _ => false,
                },
                CellOperation::NumOfBytesOfPayload => match btree_type {
                    BTreePageHeaderFormat::LeafTableBTreePage
                    | BTreePageHeaderFormat::LeafIndexBTreePage
                    | BTreePageHeaderFormat::InteriorIndexBTreePage => true,
                    _ => false,
                },
                CellOperation::Rowid => match btree_type {
                    BTreePageHeaderFormat::LeafTableBTreePage
                    | BTreePageHeaderFormat::InteriorTableBTreePage => true,
                    _ => false,
                },
                CellOperation::Payload => match btree_type {
                    BTreePageHeaderFormat::LeafTableBTreePage
                    | BTreePageHeaderFormat::LeafIndexBTreePage
                    | BTreePageHeaderFormat::InteriorIndexBTreePage => true,
                    _ => false,
                },
                CellOperation::PageNumOfFirstOverflowPage => match btree_type {
                    BTreePageHeaderFormat::LeafTableBTreePage
                    | BTreePageHeaderFormat::LeafIndexBTreePage
                    | BTreePageHeaderFormat::InteriorIndexBTreePage => true,
                    _ => false,
                },
            };

            if is_valid {
                cell_field_map.insert(field, true);
            }
        }

        cell_field_map
    }

    pub fn get_payload_overflow_bytes(
        page_size: &usize,
        resrv_bytes_per_pg: u8,
        payload_size: usize,
        pg_type: &BTreePageHeaderFormat,
    ) -> usize {
        /*
         *   U : usable size of the db page (total page size - reserved space at the end of each
         *   page)
         *
         *    P : Payload size
         *
         *    X : max amount of payload that can be stored directly on the b-tree page without
         *       spilling onto the overflow page
         *
         *      M : min amount of payload that must be stored on the btree page before spilling is
         *           allowed.
         *
         **/

        let usable_size = page_size - resrv_bytes_per_pg as usize;

        let x;

        let is_btree_leaf_pg = match pg_type {
            BTreePageHeaderFormat::LeafTableBTreePage => true,
            _ => false,
        };

        if is_btree_leaf_pg {
            x = usable_size - 35;
        } else {
            x = ((usable_size - 12) * 64 / 255) - 23;
        }
        let m = (((usable_size - 12) * 32) / 255) - 23;
        let k = m + ((payload_size - m) % (usable_size - 4));

        if payload_size <= x {
            return 0;
        } else {
            if k <= x {
                return payload_size - k;
            } else {
                return payload_size - m;
            }
        }
    }
}
