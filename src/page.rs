use crate::{cell::Cell, file::enums::BTreePageHeaderFormat};

#[derive(Debug)]
pub struct PageHeader {
    // All the page headers are in big endian.
    pub btree_page_type: BTreePageHeaderFormat, // 1 byte
    pub first_freeblock_start: u16,
    pub num_of_cells: u16,

    // cc -> cell content
    pub start_ccarea: u16,
    pub frag_ccarea: u8, // frag: fragmented

    pub rightmost_ptr: Option<u32>, // Only appears for interior b-tree pages
}

#[derive(Debug)]
pub struct Page {
    // A page consists of database header (only in the 1st page), page header, cell pointer array,
    // unallocated space, cell content area and the reserved region. This struct only consist of
    // page header and cells (Not an overall representation of the whole page, but only for us to
    // keep the important data, that we need).
    pub page_header: Option<PageHeader>,
    pub cells: Vec<Cell>,
}

