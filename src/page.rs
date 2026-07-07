use std::collections::HashMap;

use crate::{
    cell::{Cell, CellOperation},
    file::{
        DBFile,
        enums::{BTreePageHeaderFormat, TxtEncoding},
    },
    recordformat::{RecordDataType, RecordFormat, get_enconding_type},
    utils::{parse_be_byte_to_int, parse_varint_to_int},
};

use super::custom_error::CustomError;

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
pub struct BtreePage {
    // A page consists of database header (only in the 1st page), page header, cell pointer array,
    // unallocated space, cell content area and the reserved region. This struct only consist of
    // page header and cells (Not an overall representation of the whole page, but only for us to
    // keep the important data, that we need).
    pub page_header: Option<PageHeader>,
    pub cells: Vec<Cell>,
}

#[derive(Debug)]
pub struct OverflowPage {
    pub nextpgno: u32,
    pub content: Vec<RecordDataType>,
}

#[derive(Debug)]
pub struct OverflowHashmap {
    pub prevpg: u32,
    pub nextpg: u32,
    pub parent_pgno: u32,
    pub cellno: u16,

    // Idx of the content in the cell payload for which overflow is happening.
    pub cont_payload_idx: u32,

    // Remaining bytes for the content for which we are overflowing.
    pub cont_remaining_bytes: u32,
}

#[derive(Debug)]
pub enum PageType {
    Btree(BtreePage),

    // when overflow happens in a cell rest of the content is stored in linkedin list. Where
    // each node is a overflow page consisting of the nextpg idx and the content.
    Overflow(OverflowPage),
}

#[derive(Debug)]
pub enum PageKind {
    Btree,
    Overflow,
}

pub trait Page {
    fn read_page(
        &mut self,
        pgno: usize,
        page_start: usize,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn get_pgtype(&self, pgno: usize) -> PageKind;

    fn get_btree_page_type(&self, offset_val: u8) -> Result<BTreePageHeaderFormat, CustomError>;

    fn read_btree_page(
        &mut self,
        pgno: usize,
        page_start: usize,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn read_overflow_page(
        &mut self,
        pgno: usize,
        pgstart: usize,
        pgend: usize,
        enc_type: TxtEncoding,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

impl Page for DBFile {
    // Read page header and cell content.
    fn read_page(
        &mut self,
        pgno: usize,
        page_start: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self.get_pgtype(pgno) {
            PageKind::Btree => self.read_btree_page(pgno, page_start)?,
            _ => {
                let mut pgstart = 0;
                let mut pgend = 0;
                let mut enc_type = TxtEncoding::UTF8;

                if let Some(header) = &self.db_header {
                    pgstart = (pgno - 1) * header.page_size as usize;
                    pgend = pgstart + header.page_size as usize;
                    enc_type = get_enconding_type(header.enc_val);
                }

                self.read_overflow_page(pgno, pgstart, pgend, enc_type)?
            }
        }

        Ok(())
    }

    fn read_overflow_page(
        &mut self,
        pgno: usize,
        pgstart: usize,
        pgend: usize,
        enc_type: TxtEncoding,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let overflowpg_info = self
            .overflowpg
            .get(&pgno)
            .expect("This is already set previously either in btree page or in overflow page");

        let parent_pgno = overflowpg_info.parent_pgno;
        let parent_cellno = overflowpg_info.cellno;
        let mut cont_payload_idx = overflowpg_info.cont_payload_idx;

        // Parent page must be a btree page, since it contains all the cell header and payload
        // info.
        let PageType::Btree(parent_page) = &self.pages[(parent_pgno - 1) as usize] else {
            return Ok(());
        };

        let parent_cell = &parent_page.cells[parent_cellno as usize];

        let parent_payload = parent_cell
            .payload
            .as_ref()
            .expect("payload should be set in read btree function");

        let cont_payload_header = &parent_payload.rows[cont_payload_idx as usize];

        let buf = &self.buf[pgstart..pgend];

        // If > 0 mean we have more overflow and if 0 it means this is the last overflow page of
        // the cell.
        let nxt_pgno = parse_be_byte_to_int::<u32>(buf, 0);

        // First 4 bytes are reserved for the idx of next page.
        let cur_pg_content_size = buf.len() - 4;

        // Remaining bytes for the content for which we are overflowing.
        let mut cont_remaining_bytes = overflowpg_info.cont_remaining_bytes as usize;
        let mut record_format = RecordFormat::new();

        let mut rows = vec![];
        let mut new_cont_payload_idx = 0;

        let content;

        let endoffset = if cont_remaining_bytes > 0 && cont_remaining_bytes < cur_pg_content_size {
            cont_remaining_bytes
        } else {
            cur_pg_content_size
        };

        content = record_format.get_content(
            &buf[4..endoffset],
            cont_payload_header.header.0,
            &enc_type,
        )?;

        rows.push(content);

        if cur_pg_content_size > cont_remaining_bytes {
            let buf_offset = 4 + cont_remaining_bytes;

            // Since we have already read the content of remaining bytes of the continuted cell
            // content. So none remains and we move forward for the next content.
            cont_remaining_bytes = 0;

            // We have to skip the no. of elements which are after
            // our cont payload idx.
            let payload_header: Vec<(u64, usize)> = parent_payload
                .rows
                .iter()
                .skip((cont_payload_idx + 1) as usize)
                .map(|val| val.header)
                .collect();

            record_format.set_content(
                &buf[buf_offset as usize..],
                &enc_type,
                &mut new_cont_payload_idx,
                payload_header,
                &mut rows,
                &mut (cont_remaining_bytes as u32),
            )?;

            cont_payload_idx = new_cont_payload_idx;
        } else if cur_pg_content_size < cont_remaining_bytes {
            cont_remaining_bytes = cont_remaining_bytes - cur_pg_content_size;
        }

        self.overflowpg.insert(
            nxt_pgno as usize,
            OverflowHashmap {
                prevpg: pgno as u32,
                nextpg: 0,
                parent_pgno,
                cellno: parent_cellno,
                cont_payload_idx,
                cont_remaining_bytes: cont_remaining_bytes as u32,
            },
        );

        self.pages.push(PageType::Overflow(OverflowPage {
            nextpgno: nxt_pgno as u32,
            content: rows,
        }));

        self.overflowpg
            .entry(pgno)
            .and_modify(|val| val.nextpg = nxt_pgno);

        Ok(())
    }

    fn get_pgtype(&self, pgno: usize) -> PageKind {
        if self.overflowpg.get(&pgno).is_none() {
            return PageKind::Btree;
        };

        return PageKind::Overflow;
    }

    // In the page header format table it's mentioned, for which offset
    // what is the type of b-tree page we have.
    fn get_btree_page_type(&self, offset_val: u8) -> Result<BTreePageHeaderFormat, CustomError> {
        match offset_val {
            2 => Ok(BTreePageHeaderFormat::InteriorIndexBTreePage),
            5 => Ok(BTreePageHeaderFormat::InteriorTableBTreePage),
            10 => Ok(BTreePageHeaderFormat::LeafIndexBTreePage),
            13 => Ok(BTreePageHeaderFormat::LeafTableBTreePage),
            _ => Err(CustomError::InvalidOffsetValueError),
        }
    }

    // fn read_btree_page(){}
    fn read_btree_page(
        &mut self,
        pgno: usize,
        page_start: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
        for idx in 0..total_cellptr {
            let mut cell = Cell {
                page_num_of_left_child: None,
                payload_size: 0,
                rowid: None,
                payload: None,
                first_overflow_pgno: 0,
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
                cell.payload_size = payload_size;

                cur_cc_offset += payload_varint_size;
            }

            let mut rowid: u64 = 0;
            if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Rowid) {
                let rowid_varint_size = parse_varint_to_int(&self.buf[cur_cc_offset..], &mut rowid);
                cell.rowid = Some(rowid);
                cur_cc_offset += rowid_varint_size;
            }

            // If a btree has a payload then only it can have overflow (hence we are using a single
            // cell operation for both).
            if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Payload) {
                let mut record_format = RecordFormat::new();

                // This overflow bytes is for the whole of the payload
                let mut overflow_bytes: usize = 0;

                let mut encoding_type: TxtEncoding = TxtEncoding::UTF8;
                let mut cont_payload_idx = 0;

                let mut cont_remaining_bytes = 0;

                if let Some(db_header) = &self.db_header {
                    let page_size = db_header.page_size as usize;
                    encoding_type = get_enconding_type(db_header.enc_val);

                    // Get overflow bytes (tells us how much bytes in the payload vs how many in the overflow
                    // linked list). 0 means that there is no overflow
                    overflow_bytes = CellOperation::get_payload_overflow_bytes(
                        &page_size,
                        db_header.resrv_bytes_per_pg,
                        payload_size as usize,
                        &btree_page_type,
                    );

                    if overflow_bytes > 0 {
                        payload_size = payload_size - overflow_bytes as u64;
                    }
                }

                let buf_slice = &self.buf[cur_cc_offset..cur_cc_offset + payload_size as usize];
                record_format.set_records(
                    buf_slice,
                    &encoding_type,
                    &mut cont_payload_idx,
                    &mut cont_remaining_bytes,
                )?;

                cur_cc_offset += buf_slice.len();
                cell.payload = Some(record_format);

                if overflow_bytes > 0 {
                    let first_overflow_page_num =
                        parse_be_byte_to_int::<u32>(&self.buf, cur_cc_offset);
                    cell.first_overflow_pgno = first_overflow_page_num;

                    //  The idea here is to have easy links b/w the overflow pages and the
                    //  parent page (which contains the cell header and payload info)
                    self.overflowpg.insert(
                        first_overflow_page_num as usize,
                        OverflowHashmap {
                            prevpg: pgno as u32,
                            nextpg: 0,
                            parent_pgno: pgno as u32,
                            cellno: idx,
                            cont_payload_idx,
                            cont_remaining_bytes,
                        },
                    );
                }
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

        self.pages.push(PageType::Btree(BtreePage {
            page_header: Some(page_header),
            cells,
        }));

        Ok(())
    }
}
