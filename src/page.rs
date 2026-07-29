use std::error::Error;

use crate::{
    btree::{
        BTreePageHeaderFormat, Child, ChildPayload, DBHeader, InteriorIndexPayload,
        InteriorTablePayload, LeafPayload, Root, RootPayload, SqlSchema,
    },
    cell::Cell,
    file::enums::TxtEncoding,
    schema::{RecordDataType, RootSchema, Schema},
    utils::{parse_be_byte_to_int, read_specific_bytes},
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
        let nextpg = parse_be_byte_to_int!(pg_raw_bytes, 0, u32);

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
    ) -> Result<(PageHeader, Vec<Cell>), Box<dyn Error>> {
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

        Ok((pgheader, cells))
    }

    fn get_interior_node_ptr(
        &self,
        pgheader: &PageHeader,
        cells: &Vec<Cell>,
    ) -> Vec<InteriorTablePayload> {
        // An interior page contains K keys together with K+1 ptr's

        let mut interior_payload: Vec<InteriorTablePayload> = vec![];

        // Setting all the left pointers and keys
        for cell in cells {
            if cell.pgnum_of_left_child.is_some() {
                interior_payload.push(InteriorTablePayload {
                ptr: cell.pgnum_of_left_child.expect("ptr to child nodes will be there cause this function will only be called for interior nodes"),
                key: cell.rowid,
                });
            }
        }

        // Setting rightmost ptr at the end.
        if pgheader.rightmost_ptr.is_some() {
            interior_payload.push(InteriorTablePayload {
                ptr: pgheader.rightmost_ptr.unwrap(),
                key: None,
            });
        }

        interior_payload
    }

    type PgDataReturnType;

    fn get_pgdata(
        &self,
        dbheader: &DBHeader,
        pgheader: &PageHeader,
        payload: &Vec<Cell>,
    ) -> Self::PgDataReturnType;
}

impl Page for Root {
    type PgDataReturnType = Result<RootPayload, Box<dyn Error>>;

    fn get_pgdata(
        &self,
        dbheader: &DBHeader,
        pgheader: &PageHeader,
        cells: &Vec<Cell>,
    ) -> Self::PgDataReturnType {
        // Since this is for the root, it can have only 2 possible btree types Interior table or Leaf
        // Because the root has to hold the schema or nodes that point to the schema
        if pgheader.btree_pgtype == BTreePageHeaderFormat::InteriorTableBTreePage {
            Ok(RootPayload::InteriorTable(
                self.get_interior_node_ptr(pgheader, cells),
            ))
        } else {
            let mut sqlschema_list: Vec<SqlSchema> = vec![];

            for cell in cells {
                let sql_schema: SqlSchema = self.extract_schema(
                    dbheader.enc_val,
                    cell.payload
                        .as_ref()
                        .expect("payload is set for all the btree  pages except interior table"),
                )?;
                sqlschema_list.push(sql_schema);
            }
            Ok(RootPayload::RootLeafTable(sqlschema_list))
        }
    }
}

impl Page for Child {
    type PgDataReturnType = Result<ChildPayload, Box<dyn Error>>;

    fn get_pgdata(
        &self,
        dbheader: &DBHeader,
        pgheader: &PageHeader,
        cells: &Vec<Cell>,
    ) -> Self::PgDataReturnType {
        if pgheader.btree_pgtype == BTreePageHeaderFormat::InteriorTableBTreePage {
            Ok(ChildPayload::InteriorTablePayload(
                self.get_interior_node_ptr(pgheader, cells),
            ))
        } else if pgheader.btree_pgtype == BTreePageHeaderFormat::InteriorTableBTreePage {
            let child_node_ptr = self.get_interior_node_ptr(pgheader, cells);

            let mut payload: Vec<InteriorIndexPayload> = vec![];
            for (idx, cell) in cells.iter().enumerate() {
                let data = self.read_content(
                    dbheader.enc_val,
                    cell.payload
                        .as_ref()
                        .expect("payload is set for all the btree  pages except interior table"),
                )?;

                if cell.rowid.is_some() {
                    payload.push(InteriorIndexPayload {
                        ptr: child_node_ptr[idx].ptr,
                        data: Some(data),
                    });
                }
            }

            let rightmost_ptr = child_node_ptr[child_node_ptr.len() - 1].ptr;

            payload.push(InteriorIndexPayload {
                ptr: rightmost_ptr,
                data: None,
            });

            Ok(ChildPayload::InteriorIndexPayload(payload))
        } else {
            let mut leaftable_payload: Vec<LeafPayload> = vec![];
            for cell in cells {
                let data = self.read_content(
                    dbheader.enc_val,
                    cell.payload
                        .as_ref()
                        .expect("payload is set for all the btree  pages except interior table"),
                )?;

                if cell.rowid.is_some() {
                    leaftable_payload.push(LeafPayload {
                        rowid: cell.rowid,
                        data,
                    });
                }
            }

            if pgheader.btree_pgtype == BTreePageHeaderFormat::LeafTableBTreePage {
                return Ok(ChildPayload::LeafTablePayload(leaftable_payload));
            }

            Ok(ChildPayload::LeafIndexPayload(leaftable_payload))
        }
    }
}
