use crate::{
    file::{DBHeader, enums::BTreePageHeaderFormat},
    recordformat::RecordFormat,
};

pub enum CellOperation {
    PageNumLeftChild,
    NumOfBytesOfPayload,
    Rowid,
    Payload,
    PageNumOfFirstOverflowPage,
}

#[derive(Debug)]
pub struct Cell {
    pub page_num_of_left_child: Option<u32>,
    pub payload_size: u64,
    pub rowid: Option<u64>,
    pub payload: Option<RecordFormat>,
    pub first_overflow_pgno: u32,
}

impl CellOperation {
    pub fn cell_format_validator(
        btree_type: &BTreePageHeaderFormat,
        operation_type: CellOperation,
    ) -> bool {
        // Check the bcell format in here -> https://www.sqlite.org/fileformat.html#the_database_header
        match operation_type {
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
        }
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
