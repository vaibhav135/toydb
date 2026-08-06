use std::{collections::HashMap, error::Error};

use crate::{
    btree::{DBHeader, InteriorTablePayload, SqlSchema},
    page::{Page, PageHeader},
    schema::RecordDataType,
};

#[derive(Debug)]
pub struct InteriorIndexPayload {
    pub ptr: u32,
    pub data: Option<Vec<RecordDataType>>,
}

#[derive(Debug, Default)]
// Btw root also have leaf table, but since the structure is already
// defined i.e., sql schema. So create a more rigid struct there.
// Also since all we need is the list of data for leaf which is common for
// both the leaf index and leaf table.
pub struct LeafPayload {
    pub rowid: Option<u64>,
    pub row: Vec<RecordDataType>,
}

#[derive(Debug)]
pub enum ChildPayload {
    InteriorTablePayload(Vec<InteriorTablePayload>),
    InteriorIndexPayload(Vec<InteriorIndexPayload>),
    LeafTablePayload(Vec<LeafPayload>),
    LeafIndexPayload(Vec<LeafPayload>),
}

impl Default for ChildPayload {
    fn default() -> Self {
        ChildPayload::LeafTablePayload(vec![])
    }
}

#[derive(Debug, Default)]
pub struct TableRow {
    pub tblname: String,
    // First tuple is the rowid and at second pos it's the row.
    pub rows: Vec<(Option<u64>, Vec<RecordDataType>)>,
    pub total_rows: u64,
}

#[derive(Debug, Default)]
pub struct Child {
    pgno: u32,
    pgheader: PageHeader,
    data: ChildPayload,
}

impl Child {
    pub fn get_rows(
        &self,
        filepath: &String,
        dbheader: &DBHeader,
        schema: &SqlSchema,
    ) -> Result<TableRow, Box<dyn Error>> {
        let rootpg = schema.rootpg;

        let mut tablerow = TableRow {
            tblname: schema.tbl_name.to_owned(),
            rows: vec![],
            total_rows: 0,
        };

        self.get_child_data(filepath, dbheader, rootpg, schema, &mut tablerow)?;

        Ok(tablerow)
    }

    fn get_child_data(
        &self,
        filepath: &String,
        dbheader: &DBHeader,
        pgno: u32,
        schema: &SqlSchema,
        tablerow: &mut TableRow,
    ) -> Result<(), Box<dyn Error>> {
        let pgsize = dbheader.page_size;
        let pgoffset = (pgno - 1) * pgsize as u32;

        println!("pgsize: {}", pgsize);
        println!("pgno: {}", pgno);
        println!("in child: {}", pgoffset);

        let (pgheader, cells) = self.read_page(filepath, dbheader, 0, pgoffset)?;

        let pgdata = self.get_pgdata(&dbheader, &pgheader, &cells)?;

        match pgdata {
            ChildPayload::LeafTablePayload(leafpayload) => {
                for payload in leafpayload {
                    tablerow.rows.push((payload.rowid, payload.row));
                    tablerow.total_rows += 1;
                }
            }
            ChildPayload::InteriorTablePayload(interior_payload) => {
                for payload in interior_payload {
                    let nxtpgno = payload.ptr;

                    self.get_child_data(filepath, dbheader, nxtpgno, schema, tablerow);
                }
            }
            ChildPayload::LeafIndexPayload(payload) => {}
            ChildPayload::InteriorIndexPayload(payload) => {}
        }

        Ok(())
    }
}
