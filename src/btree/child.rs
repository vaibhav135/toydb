/**
*  List of imp links:
*     
*     doc on sqlite limits -> https://sqlite.org/limits.html
*
* */
use std::{error::Error, ops::Deref};

use crate::{
    btree::{
        DBHeader, InteriorTablePayload, SchemaType, SqlSchema, interior_binsearch,
        interior_idx_binsearch_by_val, leaf_binsearch, leaf_idx_binsearch_by_val,
    },
    page::{Page, PageHeader},
    query::QueryOperations,
    record_type::RecordDataType,
};

#[derive(Debug, Clone)]
pub struct InteriorIndexPayload {
    pub leftptr: u32,
    pub rightptr: u32,
    pub data: Option<Vec<RecordDataType>>,
}

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Default, Clone)]
pub struct TableRow {
    pub tblname: String,
    // First tuple is the rowid and at second pos it's the row.
    pub rows: Vec<(Option<u64>, Vec<RecordDataType>)>,
    pub total_rows: u64,
}

#[derive(Debug)]
pub enum Row {
    INDEX(IndexRow),
    TABLE(TableRow),
}

#[derive(Debug, Default, Clone)]
pub struct IndexRow {
    // First tuple is the rowid and at second pos it's the row.
    pub rows: Vec<u64>,
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
        queryop: QueryOperations,
    ) -> Result<Row, Box<dyn Error>> {
        let rootpg = schema.rootpg;

        match schema.schema_type {
            SchemaType::TABLE => {
                let mut tablerow = TableRow {
                    tblname: schema.tbl_name.to_owned(),
                    rows: vec![],
                    total_rows: 0,
                };

                return Ok(Row::TABLE(self.get_child_data(
                    filepath,
                    dbheader,
                    rootpg,
                    schema,
                    &mut tablerow,
                    &queryop,
                )?));
            }
            SchemaType::INDEX => {
                let mut indexrow = IndexRow {
                    rows: vec![],
                    total_rows: 0,
                };
                return Ok(Row::INDEX(self.get_child_indices(
                    filepath,
                    dbheader,
                    rootpg,
                    schema,
                    &mut indexrow,
                    &queryop,
                )?));
            }
            _ => Err(format!("").into()),
        }

        // Ok(tablerow)
    }

    fn get_child_data(
        &self,
        filepath: &String,
        dbheader: &DBHeader,
        pgno: u32,
        schema: &SqlSchema,
        tablerow: &mut TableRow,
        queryop: &QueryOperations,
    ) -> Result<TableRow, Box<dyn Error>> {
        let pgsize = dbheader.page_size;
        let pgoffset = (pgno - 1) * pgsize as u32;

        // println!("pgsize: {}", pgsize);
        // println!("pgno: {}", pgno);
        // println!("in child: {}", pgoffset);

        let (pgheader, cells) = self.read_page(filepath, dbheader, 0, pgoffset)?;

        let pgdata = self.get_pgdata(&dbheader, &pgheader, &cells)?;

        match pgdata {
            ChildPayload::LeafTablePayload(leafpayload) => match queryop {
                QueryOperations::SearchByID(id) => {
                    let res: Option<LeafPayload> =
                        leaf_binsearch!(leafpayload, id.to_owned(), u64, |p: &LeafPayload| p
                            .rowid
                            .unwrap());

                    if let Some(data) = res {
                        tablerow.rows.push((data.rowid, data.row));
                    }

                    Ok(tablerow.clone())
                }
                QueryOperations::GetAll => {
                    for payload in leafpayload {
                        tablerow.rows.push((payload.rowid, payload.row));
                        tablerow.total_rows += 1;
                    }

                    Ok(tablerow.clone())
                }
                _ => Err(format!("Internal Error: Invalid query operation!!!").into()),
            },
            ChildPayload::InteriorTablePayload(interior_payload) => {
                match queryop {
                    QueryOperations::SearchByID(id) => {
                        // This could mean 2 things either the user is searching by some id itself,
                        // or the field is indexed and we got the id from there.
                        // The role of this operation interior table is get to the right leaf
                        // page. In order to find the data that belongs to that specific id.

                        // let pgptr = interior_tbl_binsearch_for_id(interior_payload, id.to_owned());
                        let pgptr = interior_binsearch!(
                            interior_payload,
                            id.to_owned(),
                            |p: &InteriorTablePayload| p.key
                        );

                        if pgptr == 0 {
                            // Interior file will always return a valid ptr if it can't
                            // then something is wrong with the file, but it always should
                            return Err(format!("Invalid db file!!!").into());
                        };

                        self.get_child_data(filepath, dbheader, pgptr, schema, tablerow, queryop);

                        Ok(tablerow.clone())
                    }
                    QueryOperations::GetAll => {
                        for (idx, payload) in interior_payload.iter().enumerate() {
                            let nxtpgno = payload.leftptr;

                            self.get_child_data(
                                filepath, dbheader, nxtpgno, schema, tablerow, queryop,
                            );

                            if idx == interior_payload.len() - 1 {
                                // This is for the rightmost ptr.
                                self.get_child_data(
                                    filepath,
                                    dbheader,
                                    payload.rightptr,
                                    schema,
                                    tablerow,
                                    queryop,
                                );
                            }
                        }

                        Ok(tablerow.clone())
                    }
                    _ => Err(format!("Internal Error: Invalid query operation!!!").into()),
                }
            }

            _ => Err(format!("Internal Error: !!!").into()),
        }
    }

    fn get_child_indices(
        &self,
        filepath: &String,
        dbheader: &DBHeader,
        pgno: u32,
        schema: &SqlSchema,
        indexrow: &mut IndexRow,
        queryop: &QueryOperations,
    ) -> Result<IndexRow, Box<dyn Error>> {
        let pgsize = dbheader.page_size;
        let pgoffset = (pgno - 1) * pgsize as u32;

        // println!("pgsize: {}", pgsize);
        // println!("pgno: {}", pgno);
        // println!("in child: {}", pgoffset);

        let (pgheader, cells) = self.read_page(filepath, dbheader, 0, pgoffset)?;

        let pgdata = self.get_pgdata(&dbheader, &pgheader, &cells)?;

        match pgdata {
            ChildPayload::LeafIndexPayload(payload) => match queryop {
                QueryOperations::IdxSearchByVal(data) => {
                    if let Some(res) = leaf_idx_binsearch_by_val(payload, data.to_owned()) {
                        let tbl_id: i64 = res.row[1].to_owned().into();
                        indexrow.rows.push(tbl_id as u64);
                        indexrow.total_rows += 1;
                    }
                    Ok(indexrow.clone())
                }
                _ => Err(format!("Internal Error: Invalid query operation!!!").into()),
            },
            ChildPayload::InteriorIndexPayload(payload) => match queryop {
                QueryOperations::IdxSearchByVal(data) => {
                    let pgptr = interior_idx_binsearch_by_val(payload, data.to_owned());
                    self.get_child_indices(filepath, dbheader, pgptr, schema, indexrow, queryop);
                    Ok(indexrow.clone())
                }
                _ => Err(format!("Internal Error: Invalid query operation!!!").into()),
            },
            _ => Err(format!("Internal Error: Invalid query operation!!!").into()),
        }
    }
}
