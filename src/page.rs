use std::error::Error;

use crate::{
    btree::{DBHeader, Root, RootPayload},
    cell::Cell,
    file::enums::BTreePageHeaderFormat,
    utils::{parse_be_byte_to_int, parse_le_byte_to_int, read_specific_bytes},
};

use super::custom_error::CustomError;

#[derive(Debug)]
pub struct PageHeader {
    // All the page headers are in big endian.
    pub btree_pgtype: BTreePageHeaderFormat, // 1 byte
    pub first_freeblock_start: u16,
    pub num_of_cells: u16,

    // cc -> cell content
    pub ccarea_start: u16,

    //Number of fragmented freebytes in cell content areea
    pub ff_bytes_ccarea: u8,
    pub rightmost_ptr: Option<u32>, // Only appears for interior b-tree pages
    pub pgheader_size: u8,
}

impl Default for PageHeader {
    fn default() -> Self {
        PageHeader {
            btree_pgtype: BTreePageHeaderFormat::LeafTableBTreePage,
            first_freeblock_start: 0,
            num_of_cells: 0,
            ccarea_start: 0,
            ff_bytes_ccarea: 0,
            rightmost_ptr: None,
            pgheader_size: 0,
        }
    }
}

// #[derive(Debug)]
// pub struct BtreePage {
//     // A page consists of database header (only in the 1st page), page header, cell pointer array,
//     // unallocated space, cell content area and the reserved region. This struct only consist of
//     // page header and cells (Not an overall representation of the whole page, but only for us to
//     // keep the important data, that we need).
//     pub page_header: Option<PageHeader>,
//     pub cells: Vec<Cell>,
// }

// #[derive(Debug)]
// pub struct OverflowPage {
//     pub nextpgno: u32,
//     pub content: Vec<RecordDataType>,
// }
//
// #[derive(Debug)]
// pub struct OverflowHashmap {
//     pub prevpg: u32,
//     pub nextpg: u32,
//     pub parent_pgno: u32,
//     pub cellno: u16,
//
//     // Idx of the content in the cell payload for which overflow is happening.
//     pub cont_payload_idx: u32,
//
//     // Remaining bytes for the content for which we are overflowing.
//     pub cont_remaining_bytes: u32,
// }

// #[derive(Debug)]
// pub enum PageType {
//     Btree(BtreePage),
//
//     // when overflow happens in a cell rest of the content is stored in linkedin list. Where
//     // each node is a overflow page consisting of the nextpg idx and the content.
//     Overflow(OverflowPage),
// }
//
// #[derive(Debug)]
// pub enum PageKind {
//     Btree,
//     Overflow,
// }

pub trait Page {
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

    // if it's an interior page it will have extra four bytes in the page header.
    fn is_interior_page(&self, btree_pgtype: &BTreePageHeaderFormat) -> bool {
        // In B-tree page header if the btree is a interior page type we will have extra
        // 4 bytes in the end of the page header.
        match btree_pgtype {
            BTreePageHeaderFormat::InteriorIndexBTreePage
            | BTreePageHeaderFormat::InteriorTableBTreePage => true,
            _ => false,
        }
    }

    fn read_pgheader(&self, buf: &[u8]) -> Result<PageHeader, Box<dyn Error>> {
        // This is the first element of the page header size: 1 byte, offset: 0
        let btree_pgtype = self.get_btree_page_type(buf[0])?;

        let first_freeblock_start = parse_be_byte_to_int!(buf, 1, u16);

        let num_of_cells = parse_be_byte_to_int!(buf, 3, u16);

        let ccarea_start = parse_be_byte_to_int!(buf, 5, u16);

        // fragmented free bytes in cell content area
        let ff_bytes_ccarea = parse_be_byte_to_int!(buf, 7, u8);

        let mut rightmost_ptr = None;
        let mut pgheader_size = 8;

        if self.is_interior_page(&btree_pgtype) {
            rightmost_ptr = Some(parse_be_byte_to_int!(buf, 8, u32));
            pgheader_size = 12;
        }

        Ok(PageHeader {
            btree_pgtype,
            first_freeblock_start,
            num_of_cells,
            ccarea_start,
            ff_bytes_ccarea,
            rightmost_ptr,
            pgheader_size,
        })
    }

    fn get_pgcells(
        &self,
        pgheader: &PageHeader,
        dbheader: &DBHeader,
        buf: &[u8],
        start_byte: u8,
    ) -> Vec<Cell> {
        let cellptr_arr = Cell::get_cellptr_arr(pgheader.num_of_cells, buf);
        let mut cells = vec![];

        for cellptr in cellptr_arr {
            let mut cell = Cell::new();
            cell.read_cell(
                &pgheader,
                dbheader,
                (cellptr as u8 - start_byte) as usize,
                buf,
            );

            cells.push(cell);
        }

        cells
    }

    fn read_overflowpg(&self, curr_payload: &mut Vec<u8>, pgno: u32, pgsize: u8, buf: &[u8]) {
        let pgoffset = (pgno - 1) * pgsize as u32;

        // This is your overflow page with next pg ptr..
        let pg_raw_bytes = &buf[(pgoffset) as usize..(pgoffset + pgsize as u32) as usize];

        // First four bytes make the address of next overflow pg. If 0 then no overflowpg.
        let nextpg = parse_le_byte_to_int!(pg_raw_bytes, 0, u32);

        // Overflow pg excluding the next pg ptr.
        let additional_payload_bytes = &pg_raw_bytes[4..];
        curr_payload.extend_from_slice(additional_payload_bytes);

        if nextpg > 0 {
            self.read_overflowpg(curr_payload, pgno, pgsize, buf);
        }

        return;
    }

    fn set_payload_overflow_bytes(&self, cells: &mut Vec<Cell>, buf: &[u8], pgsize: u8) {
        for cell in cells {
            if let Some(payload) = &mut cell.payload {
                if cell.first_overflow_pgno > 0 {
                    self.read_overflowpg(payload, cell.first_overflow_pgno, pgsize, buf);
                }
            }
        }
    }

    fn read_page(
        &self,
        filepath: &String,
        dbheader: &DBHeader,
        pgsize: u8,
        start_offset: u8,
    ) -> Result<(), Box<dyn Error>> {
        let page_raw_bytes = read_specific_bytes(
            filepath,
            start_offset as u16,
            (pgsize - start_offset) as u16,
        )?;

        let pgheader = self.read_pgheader(&page_raw_bytes)?;

        let mut cells = self.get_pgcells(&pgheader, &dbheader, &page_raw_bytes, start_offset);

        if pgheader.btree_pgtype != BTreePageHeaderFormat::InteriorTableBTreePage {
            self.set_payload_overflow_bytes(&mut cells, &page_raw_bytes, pgsize);
        }

        /*
         *   So what to do now???
         *
         *   what do we have -> I mean we have either data of
         *      leaf table      ,    leaf index
         *      interior table  ,    interior index
         *
         *      leaf table      ->   check for overflow, handle overflow, append extrabytes then
         *                           extract payload. If it's root then store it as schmea sql.
         *                           Otherwiseit's a row data (the structure for that we can think).
         *
         *
         *      interior table  ->   There is no overflow. Only left chlid and rowid, and right most
         *                           pointer. First gather all the pointers and keys. We know they are
         *                           already sorted. Then once we have gathered then we just need to
         *                           traverse. But again we need to perform the same operation for
         *                           check page header (basically gather data. Ofcourse we will not be
         *                           doing it for all the pages but more like for the query and related
         *                           page. But for the intital I will do it for all since I want to see
         *                           if it works well)
         *
         *       leaf index     ->   Check for overflow, Index will have again all the rows data. But
         *                           beware that we need to do index scan so searching is also
         *                           something I need to think about.
         *
         *       Interior index ->   Very similar to interior table again traverse the pointers and
         *                           store the page (that I have to think further).
         *
         *   struct InteriorTableData {
         *       ptr: u8,
         *       key: u8,
         *   }
         *
         *   struct RootLeafTable {
         *     schema
         *     sql
         *     tablename
         *   }
         *
         *   Enum RootData {
         *       InteriorTable(IntereiorPayload)
         *       LeafTable(LeafPayload)
         *   }
         *
         *   type BlahBlah =  RootData;
         *
         *
         * */

        Ok(())
    }

    type PgDataReturnType;

    fn get_pgdata(&self, pgheader: &PageHeader, payload: &Cell) -> Self::PgDataReturnType;
}

impl Page for Root {
    type PgDataReturnType = RootPayload;

    fn get_pgdata(&self, pgheader: &PageHeader, payload: &Cell) -> Self::PgDataReturnType {
        // Since this is for the root, it can have only 2 possible btree types Interior table or Leaf
        // Because the root has to hold the schema
        if pgheader.btree_pgtype == BTreePageHeaderFormat::InteriorTableBTreePage {
        } else {
        }
    }
}

// impl Page {
//     // Read page header and cell content.
//     fn read_page(
//         &mut self,
//         pgno: usize,
//         page_start: usize,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         match self.get_pgtype(pgno) {
//             PageKind::Btree => self.read_btree_page(pgno, page_start)?,
//             _ => {
//                 let mut pgstart = 0;
//                 let mut pgend = 0;
//                 let mut enc_type = TxtEncoding::UTF8;
//
//                 if let Some(header) = &self.db_header {
//                     pgstart = (pgno - 1) * header.page_size as usize;
//                     pgend = pgstart + header.page_size as usize;
//                     enc_type = get_enconding_type(header.enc_val);
//                 }
//
//                 self.read_overflow_page(pgno, pgstart, pgend, enc_type)?
//             }
//         }
//
//         Ok(())
//     }
//
//     fn read_overflow_page(
//         &mut self,
//         pgno: usize,
//         pgstart: usize,
//         pgend: usize,
//         enc_type: TxtEncoding,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         let overflowpg_info = self
//             .overflowpg
//             .get(&pgno)
//             .expect("This is already set previously either in btree page or in overflow page");
//
//         let parent_pgno = overflowpg_info.parent_pgno;
//         let parent_cellno = overflowpg_info.cellno;
//         let mut cont_payload_idx = overflowpg_info.cont_payload_idx;
//
//         // Parent page must be a btree page, since it contains all the cell header and payload
//         // info.
//         let PageType::Btree(parent_page) = &self.pages[(parent_pgno - 1) as usize] else {
//             return Ok(());
//         };
//
//         let parent_cell = &parent_page.cells[parent_cellno as usize];
//
//         let parent_payload = parent_cell
//             .payload
//             .as_ref()
//             .expect("payload should be set in read btree function");
//
//         let cont_payload_header = &parent_payload.rows[cont_payload_idx as usize];
//
//         let buf = &self.buf[pgstart..pgend];
//
//         // If > 0 mean we have more overflow and if 0 it means this is the last overflow page of
//         // the cell.
//         let nxt_pgno = parse_be_byte_to_int::<u32>(buf, 0);
//
//         // First 4 bytes are reserved for the idx of next page.
//         let cur_pg_content_size = buf.len() - 4;
//
//         // Remaining bytes for the content for which we are overflowing.
//         let mut cont_remaining_bytes = overflowpg_info.cont_remaining_bytes as usize;
//         let mut record_format = RecordFormat::new();
//
//         let mut rows = vec![];
//         let mut new_cont_payload_idx = 0;
//
//         let content;
//
//         let endoffset = if cont_remaining_bytes > 0 && cont_remaining_bytes < cur_pg_content_size {
//             cont_remaining_bytes
//         } else {
//             cur_pg_content_size
//         };
//
//         content = record_format.get_content(
//             &buf[4..endoffset],
//             cont_payload_header.header.0,
//             &enc_type,
//         )?;
//
//         rows.push(content);
//
//         if cur_pg_content_size > cont_remaining_bytes {
//             let buf_offset = 4 + cont_remaining_bytes;
//
//             // Since we have already read the content of remaining bytes of the continuted cell
//             // content. So none remains and we move forward for the next content.
//             cont_remaining_bytes = 0;
//
//             // We have to skip the no. of elements which are after
//             // our cont payload idx.
//             let payload_header: Vec<(u64, usize)> = parent_payload
//                 .rows
//                 .iter()
//                 .skip((cont_payload_idx + 1) as usize)
//                 .map(|val| val.header)
//                 .collect();
//
//             record_format.set_content(
//                 &buf[buf_offset as usize..],
//                 &enc_type,
//                 &mut new_cont_payload_idx,
//                 payload_header,
//                 &mut rows,
//                 &mut (cont_remaining_bytes as u32),
//             )?;
//
//             cont_payload_idx = new_cont_payload_idx;
//         } else if cur_pg_content_size < cont_remaining_bytes {
//             cont_remaining_bytes = cont_remaining_bytes - cur_pg_content_size;
//         }
//
//         self.overflowpg.insert(
//             nxt_pgno as usize,
//             OverflowHashmap {
//                 prevpg: pgno as u32,
//                 nextpg: 0,
//                 parent_pgno,
//                 cellno: parent_cellno,
//                 cont_payload_idx,
//                 cont_remaining_bytes: cont_remaining_bytes as u32,
//             },
//         );
//
//         self.pages.push(PageType::Overflow(OverflowPage {
//             nextpgno: nxt_pgno as u32,
//             content: rows,
//         }));
//
//         self.overflowpg
//             .entry(pgno)
//             .and_modify(|val| val.nextpg = nxt_pgno);
//
//         Ok(())
//     }
//
//     fn get_pgtype(&self, pgno: usize) -> PageKind {
//         if self.overflowpg.get(&pgno).is_none() {
//             return PageKind::Btree;
//         };
//
//         return PageKind::Overflow;
//     }
//
//
//     fn get_pgheader(&mut self, page_start: usize) -> Result<PageHeader, Box<dyn Error>> {}
//
//     // fn read_btree_page(){}
//     fn read_btree_page(
//         &mut self,
//         pgno: usize,
//         page_start: usize,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         // This is the first element of the page header size: 1 byte, offset: 0
//         let btree_page_type = self.get_btree_page_type(self.buf[page_start])?;
//
//         // In B-tree page header if the btree is a interior page type we will have extra
//         // 4 bytes in the end of the page header.
//         let has_extra_four_bytes = match btree_page_type {
//             BTreePageHeaderFormat::InteriorIndexBTreePage
//             | BTreePageHeaderFormat::InteriorTableBTreePage => true,
//             _ => false,
//         };
//
//         let btree_page_header_size: u8 = if has_extra_four_bytes { 12 } else { 8 };
//
//         let total_cellptr = parse_be_byte_to_int::<u16>(&self.buf, page_start + 3);
//         let mut cellptr_arr_offset = page_start + (btree_page_header_size as usize);
//
//         let mut cells: Vec<Cell> = vec![];
//
//         /**
//          *
//          * what do we need
//          * we need pointer array to keep left and right ptr for interior btree
//          *
//          *
//          */
//         // Each cell ptr is of 2 bytes.
//         for idx in 0..total_cellptr {
//             let mut cell = Cell {
//                 page_num_of_left_child: None,
//                 payload_size: 0,
//                 rowid: None,
//                 payload: None,
//                 first_overflow_pgno: 0,
//             };
//
//             let mut cur_cc_offset =
//                 parse_be_byte_to_int::<u16>(&self.buf, cellptr_arr_offset) as usize;
//
//             // From 2nd page the cell array ptr for cell content offset will be less
//             // than 4096, this is to keep the size in 2bytes I guess. I will update this
//             // as soon as I know the right (original) reason.
//             if page_start > cur_cc_offset {
//                 cur_cc_offset += page_start;
//             }
//
//             if CellOperation::cell_format_validator(
//                 &btree_page_type,
//                 CellOperation::PageNumLeftChild,
//             ) {
//                 let left_child_page_num = parse_be_byte_to_int::<u32>(&self.buf, cur_cc_offset);
//                 cell.page_num_of_left_child = Some(left_child_page_num);
//                 cur_cc_offset += 4;
//             }
//
//             let mut payload_size: u64 = 0;
//             if CellOperation::cell_format_validator(
//                 &btree_page_type,
//                 CellOperation::NumOfBytesOfPayload,
//             ) {
//                 let payload_varint_size =
//                     parse_varint_to_int(&self.buf[cur_cc_offset..], &mut payload_size);
//                 cell.payload_size = payload_size;
//
//                 cur_cc_offset += payload_varint_size;
//             }
//
//             let mut rowid: u64 = 0;
//             if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Rowid) {
//                 let rowid_varint_size = parse_varint_to_int(&self.buf[cur_cc_offset..], &mut rowid);
//                 cell.rowid = Some(rowid);
//                 cur_cc_offset += rowid_varint_size;
//             }
//
//             // If a btree has a payload then only it can have overflow (hence we are using a single
//             // cell operation for both).
//             if CellOperation::cell_format_validator(&btree_page_type, CellOperation::Payload) {
//                 let mut record_format = RecordFormat::new();
//
//                 // This overflow bytes is for the whole of the payload
//                 let mut overflow_bytes: usize = 0;
//
//                 let mut encoding_type: TxtEncoding = TxtEncoding::UTF8;
//                 let mut cont_payload_idx = 0;
//
//                 let mut cont_remaining_bytes = 0;
//
//                 if let Some(db_header) = &self.db_header {
//                     let page_size = db_header.page_size as usize;
//                     encoding_type = get_enconding_type(db_header.enc_val);
//
//                     // Get overflow bytes (tells us how much bytes in the payload vs how many in the overflow
//                     // linked list). 0 means that there is no overflow
//                     overflow_bytes = CellOperation::get_payload_overflow_bytes(
//                         &page_size,
//                         db_header.resrv_bytes_per_pg,
//                         payload_size as usize,
//                         &btree_page_type,
//                     );
//
//                     if overflow_bytes > 0 {
//                         payload_size = payload_size - overflow_bytes as u64;
//                     }
//                 }
//
//                 let buf_slice = &self.buf[cur_cc_offset..cur_cc_offset + payload_size as usize];
//                 record_format.set_records(
//                     buf_slice,
//                     &encoding_type,
//                     &mut cont_payload_idx,
//                     &mut cont_remaining_bytes,
//                 )?;
//
//                 cur_cc_offset += buf_slice.len();
//                 cell.payload = Some(record_format);
//
//                 if overflow_bytes > 0 {
//                     let first_overflow_page_num =
//                         parse_be_byte_to_int::<u32>(&self.buf, cur_cc_offset);
//                     cell.first_overflow_pgno = first_overflow_page_num;
//
//                     //  The idea here is to have easy links b/w the overflow pages and the
//                     //  parent page (which contains the cell header and payload info)
//                     self.overflowpg.insert(
//                         first_overflow_page_num as usize,
//                         OverflowHashmap {
//                             prevpg: pgno as u32,
//                             nextpg: 0,
//                             parent_pgno: pgno as u32,
//                             cellno: idx,
//                             cont_payload_idx,
//                             cont_remaining_bytes,
//                         },
//                     );
//                 }
//             }
//
//             // Each cell array pointer is of 2 bytes
//             cellptr_arr_offset += 2;
//
//             cells.push(cell);
//         }
//
//         let first_freeblock_start = parse_be_byte_to_int::<u16>(&self.buf, page_start + 1);
//         let cell_content_area = parse_be_byte_to_int::<u16>(&self.buf, page_start + 5);
//         let fragmented_cellcontent_area = parse_be_byte_to_int::<u8>(&self.buf, page_start + 6);
//
//         let rightmost_ptr = if has_extra_four_bytes {
//             Some(parse_be_byte_to_int::<u32>(&self.buf, page_start + 7))
//         } else {
//             None
//         };
//
//         let page_header = PageHeader {
//             btree_page_type,
//             first_freeblock_start,
//             num_of_cells: total_cellptr,
//
//             start_ccarea: cell_content_area,
//             frag_ccarea: fragmented_cellcontent_area,
//
//             rightmost_ptr,
//         };
//
//         self.pages.push(PageType::Btree(BtreePage {
//             page_header: Some(page_header),
//             cells,
//         }));
//
//         Ok(())
//     }
// }
