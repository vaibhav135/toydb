use crate::{fileformat::enums::BTreePageHeaderFormat, recordformat::RecordFormat};

pub enum CellOperation {
    PageNumLeftChild,
    NumOfBytesOfPayload,
    Rowid,
    Payload,
    PageNumOfFirstOverflowPage,
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
}

#[derive(Debug)]
pub struct Cell {
    page_num_of_left_child: Option<u32>,
    payload_size: Option<u64>,
    rowid: Option<u64>,
    payload: RecordFormat,
}
